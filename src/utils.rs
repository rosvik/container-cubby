use axum::http::{HeaderMap, HeaderValue};
use regex_lite::Regex;

pub fn insert_blob_location_header(headers: &mut HeaderMap, name: &str, digest: &str) {
  // Successful completion MUST include the following header. Location is a
  // pullable blob URL. This location does not necessarily have to be served by
  // your registry, for example, in the case of a signed URL from some cloud
  // storage provider that your registry generates.
  let blob_location = format!("/v2/{name}/blobs/{digest}");
  if let Ok(header_value) = HeaderValue::from_str(blob_location.as_str()) {
    headers.insert("Location", header_value);
  }
}

pub fn get_content_range(headers: &HeaderMap) -> Option<(String, usize, usize)> {
  let content_range = headers.get("Content-Range")?.to_str().ok()?;
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

pub fn get_content_length(headers: &HeaderMap) -> Option<usize> {
  let content_length = headers.get("Content-Length")?.to_str().ok()?;
  let length = content_length.parse::<usize>().ok()?;
  Some(length)
}
