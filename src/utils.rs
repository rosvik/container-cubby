use crate::HOST;
use crate::PROTOCOL;
use axum::http::{HeaderMap, HeaderValue};

pub fn insert_blob_location_header(headers: &mut HeaderMap, name: &str, digest: &str) {
  // Successful completion MUST include the following header. Location is a
  // pullable blob URL. This location does not necessarily have to be served by
  // your registry, for example, in the case of a signed URL from some cloud
  // storage provider that your registry generates.
  let blob_location = format!("{PROTOCOL}://{HOST}/v2/{name}/blobs/{digest}");
  headers.insert("Location", HeaderValue::from_str(blob_location.as_str()).unwrap());
}
