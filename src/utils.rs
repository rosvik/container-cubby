use actix_web::http::header::HeaderValue;
use regex_lite::Regex;
use uuid::Uuid;

pub mod ansi {
  pub const RESET: &str = "\x1b[0m";
  pub const UNDERLINE: &str = "\x1b[4m";
  pub const RED: &str = "\x1b[31m";
  pub const GREEN: &str = "\x1b[32m";
  pub const YELLOW: &str = "\x1b[93m";
  pub const BLUE: &str = "\x1b[34m";
  pub const MAGENTA: &str = "\x1b[35m";
  pub const CYAN: &str = "\x1b[36m";
  pub const ORANGE: &str = "\x1b[33m";
  pub const GRAY: &str = "\x1b[90m";
}

pub fn get_content_range(content_range: Option<&HeaderValue>) -> Option<(String, usize, usize)> {
  let content_range = content_range?.to_str().ok()?;
  let range = String::from(content_range);

  // Range MUST match the regular expression `^[0-9]+-[0-9]+$`
  let re = Regex::new(r"^[0-9]+-[0-9]+$").ok()?;
  if !re.is_match(&range) {
    println!("Error: Invalid range format: {range:?}");
    return None;
  }

  let (start, end_with_dash) = range.split_at(range.find('-')?);
  let end = &end_with_dash[1..];
  let start = match start.parse::<usize>() {
    Ok(start) => start,
    Err(e) => {
      println!("Error: When parsing start={start}: {e:?}");
      return None;
    }
  };
  let end = match end.parse::<usize>() {
    Ok(end) => end,
    Err(e) => {
      println!("Error: When parsing end={end}: {e:?}");
      return None;
    }
  };

  Some((range, start, end))
}

pub fn get_content_length(content_length: Option<&HeaderValue>) -> Option<usize> {
  let length = content_length?.to_str().ok()?.parse::<usize>().ok()?;
  Some(length)
}

pub fn get_content_type(content_type: Option<&HeaderValue>) -> Option<String> {
  let content_type = content_type?.to_str().ok()?;
  // The registry SHOULD ignore parameters on the Content-Type header.
  let content_type = content_type.split(';').next()?;
  Some(content_type.to_string())
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
  pub expected: String,
  pub computed: String,
}
impl std::fmt::Debug for DigestMismatch {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "DigestMismatch {{ digest: {}, computed_digest: {} }}", self.expected, self.computed)
  }
}

#[allow(dead_code)]
pub enum Reference {
  Sha256(String),
  Tag(String),
}
/// <reference> MUST be either (a) the digest of the manifest or (b) a tag. The <reference> MUST NOT
/// be in any other format.
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pull>
pub fn verify_reference(reference: String) -> Result<Reference, ()> {
  match reference.starts_with("sha256:") {
    true => match is_safe_digest(reference.as_str()) {
      true => Ok(Reference::Sha256(reference.clone())),
      false => Err(()),
    },
    false => match is_safe_tag(reference.as_str()) {
      true => Ok(Reference::Tag(reference)),
      false => Err(()),
    },
  }
}

/// <name> MUST match the following regular expression:
/// ```regex
/// [a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*(\/[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*)*
/// ```
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pull>
pub fn is_safe_name(path: &str) -> bool {
  let re = Regex::new(r"^[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*(\/[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*)*$")
    .unwrap();
  re.is_match(path)
}

/// <reference> as a tag MUST be at most 128 characters in length and MUST match the following
/// regular expression:
/// ```regex
/// [a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}
/// ```
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pull>
pub fn is_safe_tag(reference: &str) -> bool {
  let re = Regex::new(r"^[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}$").unwrap();
  re.is_match(reference)
}

pub fn is_safe_digest(digest: &str) -> bool {
  let re = Regex::new(r"^sha256:[0-9a-f]{64}$").unwrap();
  re.is_match(digest)
}

/// The <location> MUST contain a UUID representing a unique session ID for the upload to follow.
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put>
pub fn is_safe_hunk(hunk: &str) -> bool {
  Uuid::try_parse(hunk).is_ok()
}

pub fn get_tag_range(tags: Vec<String>, count: usize, last: Option<&str>) -> Vec<String> {
  let last = last.unwrap_or("");
  let last_index = tags.iter().position(|tag| tag == last);
  match last_index {
    Some(last_index) => tags.iter().skip(last_index + 1).take(count).cloned().collect(),
    None => tags.iter().take(count).cloned().collect(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_tag_range() {
    let tags = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    assert_eq!(get_tag_range(tags.clone(), 2, None), vec!["a".to_string(), "b".to_string()]);
    assert_eq!(get_tag_range(tags.clone(), 2, Some("a")), vec!["b".to_string(), "c".to_string()]);
    assert_eq!(get_tag_range(tags.clone(), 2, Some("b")), vec!["c".to_string()]);
  }

  #[test]
  fn test_verify_reference() {
    let sha256 = "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    assert!(matches!(verify_reference(sha256.to_string()), Ok(Reference::Sha256(_))));

    let invalid_sha256 = "sha256:x94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    assert!(verify_reference(invalid_sha256.to_string()).is_err());

    let too_short_sha256 = "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde";
    assert!(verify_reference(too_short_sha256.to_string()).is_err());

    let tag = "latest";
    assert!(matches!(verify_reference(tag.to_string()), Ok(Reference::Tag(_))));

    let invalid_tag = "latest/";
    assert!(verify_reference(invalid_tag.to_string()).is_err());
  }

  #[test]
  fn test_get_content_range() {
    let content_range = &HeaderValue::from_str("0-123").unwrap();
    assert_eq!(get_content_range(Some(content_range)), Some(("0-123".to_string(), 0, 123)));

    let content_range = &HeaderValue::from_str("").unwrap();
    assert_eq!(get_content_range(Some(content_range)), None);
  }

  #[test]
  fn test_get_content_type() {
    let content_type =
      &HeaderValue::from_str("application/vnd.oci.image.manifest.v1+json").unwrap();
    assert_eq!(
      get_content_type(Some(content_type)),
      Some("application/vnd.oci.image.manifest.v1+json".to_string())
    );

    let content_type =
      &HeaderValue::from_str("application/vnd.oci.image.manifest.v1+json; param=value").unwrap();
    assert_eq!(
      get_content_type(Some(content_type)),
      Some("application/vnd.oci.image.manifest.v1+json".to_string())
    );
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

  #[test]
  fn test_is_safe_name() {
    assert!(is_safe_name("hello-world"));
    assert!(is_safe_name("hello_world"));
    assert!(is_safe_name("hello__world"));
    assert!(is_safe_name("hello.world"));
    assert!(is_safe_name("hello.world/1.3"));
    assert!(is_safe_name("hello/world/123_456"));

    assert!(!is_safe_name("hello___world"));
    assert!(!is_safe_name("hello.world/"));
    assert!(!is_safe_name("hello.world/123-"));
    assert!(!is_safe_name("hello.world/.."));
    assert!(!is_safe_name(".."));
  }

  #[test]
  fn test_is_safe_reference() {
    assert!(is_safe_tag("hello-world"));
    assert!(is_safe_tag("hello_world"));
    assert!(is_safe_tag("hello__world"));
    assert!(is_safe_tag("hello.world"));
    assert!(is_safe_tag("hello.world-1.3"));
    assert!(is_safe_tag("hello.world-123_456"));

    assert!(!is_safe_tag("hello.world/1.3"));
    assert!(!is_safe_tag("hello.world/"));
    assert!(!is_safe_tag("hello.world/123-"));
    assert!(!is_safe_tag("hello.world/.."));
    assert!(!is_safe_tag(".."));
  }

  #[test]
  fn test_is_safe_digest() {
    assert!(is_safe_digest(
      "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    ));

    assert!(!is_safe_digest(
      "sha512:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    ));
    assert!(!is_safe_digest(
      "sha256:B94D27B9934D3E08A52E52D7DA7DABFAC484EFE37A5380EE9088F7ACE2EFCDE9"
    ));
    assert!(!is_safe_digest(
      "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde"
    ));
    assert!(!is_safe_digest(
      "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde90"
    ));
    assert!(!is_safe_digest(
      "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde90"
    ));
    assert!(!is_safe_digest(
      "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9/"
    ));
  }

  #[test]
  fn test_is_safe_hunk() {
    assert!(is_safe_hunk("123e4567-e89b-12d3-a456-426614174000"));
    assert!(is_safe_hunk("35003fde-9a27-4b01-a296-1337deadbeef"));
    assert!(!is_safe_hunk("35003fde-9a27-4b01-a296-1337deadbeep"));
    assert!(!is_safe_hunk("123e4567-e89b-12d3-a456-4266141740000"));
    assert!(!is_safe_hunk("123e4567-e89b-12d3-a456-42661417400"));
  }
}
