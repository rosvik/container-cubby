use crate::digestor;
use actix_web::http::header::HeaderValue;
use regex_lite::Regex;

// pub fn insert_blob_location_header(headers: &mut HeaderMap, name: &str, digest: &str) {
//   // Successful completion MUST include the following header. Location is a
//   // pullable blob URL. This location does not necessarily have to be served by
//   // your registry, for example, in the case of a signed URL from some cloud
//   // storage provider that your registry generates.
//   let blob_location = format!("/v2/{name}/blobs/{digest}");
//   if let Ok(header_value) = HeaderValue::from_str(blob_location.as_str()) {
//     headers.insert("Location", header_value);
//   }
// }

pub fn get_content_range(content_range: Option<&HeaderValue>) -> Option<(String, usize, usize)> {
  let content_range = content_range?.to_str().ok()?;
  let range = String::from(content_range);

  // Range MUST match the regular expression `^[0-9]+-[0-9]+$`
  let re = Regex::new(r"^[0-9]+-[0-9]+$").ok()?;
  if !re.is_match(&range) {
    println!("Error: Invalid range format: {:?}", range);
    return None;
  }

  let (start, end_with_dash) = range.split_at(range.find('-')?);
  let end = &end_with_dash[1..];
  let start = match start.parse::<usize>() {
    Ok(start) => start,
    Err(e) => {
      println!("Error: When parsing start={start}: {:?}", e);
      return None;
    }
  };
  let end = match end.parse::<usize>() {
    Ok(end) => end,
    Err(e) => {
      println!("Error: When parsing end={end}: {:?}", e);
      return None;
    }
  };

  Some((range, start, end))
}

pub fn get_content_length(content_length: Option<&HeaderValue>) -> Option<usize> {
  let length = content_length?.to_str().ok()?.parse::<usize>().ok()?;
  Some(length)
}

use base64::{engine::general_purpose, Engine as _};
pub fn decode_base64(input: String) -> Result<String, Box<dyn std::error::Error>> {
  let bytes = general_purpose::STANDARD.decode(input)?;
  let utf8 = std::str::from_utf8(&bytes)?;
  Ok(utf8.to_string())
}

#[allow(dead_code)]
pub fn encode_base64(input: String) -> String {
  let bytes = input.as_bytes();
  general_purpose::STANDARD.encode(bytes)
}

pub struct DigestMismatch {
  digest: String,
  computed_digest: String,
}
impl std::fmt::Debug for DigestMismatch {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "DigestMismatch {{ digest: {}, computed_digest: {} }}",
      self.digest, self.computed_digest
    )
  }
}
pub fn verify_blob(data: &[u8], digest: &str) -> Result<(), DigestMismatch> {
  let computed_digest = digestor::get_sha256_digest(&data.to_vec());
  if computed_digest != digest {
    return Err(DigestMismatch {
      digest: digest.to_string(),
      computed_digest,
    });
  }
  Ok(())
}

#[allow(dead_code)] // TODO: Remove this once fields are used in logs
pub enum Reference<'a> {
  Sha256(&'a str),
  Tag(&'a str),
}
pub fn verify_reference(tag: &str) -> Result<Reference, ()> {
  match tag.starts_with("sha256:") {
    true => {
      let re = Regex::new(r"^sha256:[0-9a-f]{64}$").unwrap();
      if !re.is_match(tag) {
        return Err(());
      };
      return Ok(Reference::Sha256(tag));
    }
    false => {
      // <reference> as a tag MUST be at most 128 characters in length and MUST
      // match the following regular expression:
      // `[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}`
      let re = Regex::new(r"^[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}$").unwrap();
      if !re.is_match(tag) {
        return Err(());
      };
      return Ok(Reference::Tag(tag));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_verify_reference() {
    let sha256 = "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    assert!(matches!(verify_reference(sha256), Ok(Reference::Sha256(_))));

    let invalid_sha256 = "sha256:x94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    assert!(verify_reference(invalid_sha256).is_err());

    let too_long_sha256 =
      "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde90";
    assert!(verify_reference(too_long_sha256).is_err());

    let too_short_sha256 = "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde";
    assert!(verify_reference(too_short_sha256).is_err());

    let tag = "latest";
    assert!(matches!(verify_reference(tag), Ok(Reference::Tag(_))));

    let invalid_tag = "latest/";
    assert!(verify_reference(invalid_tag).is_err());
  }

  #[test]
  fn test_get_content_range() {
    let content_range = &HeaderValue::from_str("0-123").unwrap();
    assert_eq!(get_content_range(Some(content_range)), Some(("0-123".to_string(), 0, 123)));

    let content_range = &HeaderValue::from_str("").unwrap();
    assert_eq!(get_content_range(Some(content_range)), None);
  }

  #[test]
  fn test_decode_base64() {
    let input = "aGVsbG8gd29ybGQ=";
    assert_eq!(decode_base64(input.to_string()).unwrap(), "hello world");
  }

  #[test]
  fn test_encode_base64() {
    let input = "hello world";
    assert_eq!(encode_base64(input.to_string()), "aGVsbG8gd29ybGQ=");
  }
}
