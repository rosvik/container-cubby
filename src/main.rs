mod db;
mod digest;
use axum::{
  body::Bytes,
  extract::{Path, Query},
  http::{HeaderMap, HeaderValue, StatusCode},
  response::IntoResponse,
  routing::{get, post},
  Router,
};
use serde::Deserialize;

const HOST: &str = "0.0.0.0:8602";
const PROTOCOL: &str = "http";
const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
  db::init().unwrap();
  let router = router();
  let listener = tokio::net::TcpListener::bind(HOST).await.unwrap();
  println!("Listening on {PROTOCOL}://{HOST}");
  axum::serve(listener, router).await.unwrap();
}

fn router() -> Router {
  Router::new()
    .route("/", get(|| async { format!("{CRATE_NAME} v{CRATE_VERSION}") }))
    .route("/v2", get(()))
    .route("/v2/:name/blobs/:digest", get(get_blob))
    .route("/v2/:name/manifests/:reference", get(get_manifest))
    .route("/v2/:name/blobs/uploads/", post(post_blob))
}

/// end-2: `GET /v2/<name>/blobs/<digest>` => 200 / 404
///
/// RESPONSE:
/// - Docker-Content-Digest: {the blob's digest}
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-blobs
async fn get_blob(Path((name, digest)): Path<(String, String)>) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  let blob = match db::get_blob(&conn, &name, &digest) {
    Ok(blob) => blob,
    Err(e) => {
      // If the blob is not found in the registry, the response code MUST be
      // 404 Not Found.
      println!("Error getting blob: {:?}", e);
      return (StatusCode::NOT_FOUND, HeaderMap::new(), "".into());
    }
  };

  // A successful response SHOULD contain the digest of the uploaded blob in the
  // header Docker-Content-Digest. If present, the value of this header MUST be
  // a digest matching that of the response body.
  let mut success_headers = HeaderMap::new();

  success_headers
    .insert("Docker-Content-Digest", HeaderValue::from_str(blob.digest.as_str()).unwrap());

  // A GET request to an existing blob URL MUST provide the expected blob, with
  // a response code that MUST be 200 OK.
  (StatusCode::OK, success_headers, Bytes::from(blob.data.unwrap()))
}

/// end-3: `GET /v2/<name>/manifests/<reference>` => 200 / 404
///
/// REQUEST:
/// - Accept: {content type}            (see spec / content-negotiation.md)
///
/// RESPONSE:
/// - Content-Type: {content type}      (see spec / content-negotiation.md)
/// - Docker-Content-Digest: {digest}   (the canonical digest of the uploaded blob)
/// - Body: {manifest}
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests
async fn get_manifest(Path((_name, _reference)): Path<(String, String)>) {
  println!("TODO: get_manifest not implemented");
}

/// end-4b: `POST /v2/<name>/blobs/uploads/?digest=<digest>` => 201/202 / 404/400
///
/// REQUEST
/// - Content-Length: {length}          (must match the blob's actual content length)
/// - Content-Type: "application/octet-stream"
/// - Body: {blob byte stream}
///
/// RESPONSE
/// - Location: {blob-location}         (a pullable blob URL)
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#single-post
#[derive(Deserialize)]
struct PostBlobParameters {
  digest: String,
}
async fn post_blob(
  Path(name): Path<String>,
  Query(query): Query<PostBlobParameters>,
  data: Bytes,
) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  let digest = query.digest;
  let mut success_headers = HeaderMap::new();

  // Successful completion MUST include the following header. Location is a
  // pullable blob URL. This location does not necessarily have to be served by
  // your registry, for example, in the case of a signed URL from some cloud
  // storage provider that your registry generates.
  let blob_location = format!("{PROTOCOL}://{HOST}/v2/{}/blobs/{}", name, digest);
  success_headers.insert("Location", HeaderValue::from_str(blob_location.as_str()).unwrap());

  // Query digest MUST match the blob's digest.
  let blob_digest = digest::get_sha256_digest(&data.to_vec());
  if blob_digest != digest {
    println!("Digest mismatch: digest_hash_string {}, digest {}", blob_digest, digest);
    return (StatusCode::BAD_REQUEST, HeaderMap::new(), ());
  }

  match db::insert_blob(&conn, &digest, &name, &data) {
    Ok(res) => res,
    Err(e) => {
      if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
        // The blob already exists
        println!("Duplicate digest");
        return (StatusCode::CREATED, success_headers, ());
      }
      println!("Error inserting blob: {:?}", e);
      return (StatusCode::INTERNAL_SERVER_ERROR, HeaderMap::new(), ());
    }
  };

  // Successful completion of the request MUST return a 201 Created status code.
  (StatusCode::CREATED, success_headers, ())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::digest::get_sha256_digest;

  const NAMESPACE: &str = "test:1.0.0";
  const TEST_BLOB_STRING: &str = "testblob";

  #[tokio::test]
  async fn test_post_blob() {
    let test_blob_bytes: Bytes = Bytes::from(TEST_BLOB_STRING);

    let digest = get_sha256_digest(&test_blob_bytes.to_vec());

    let result =
      post_blob(Path(NAMESPACE.to_string()), Query(PostBlobParameters { digest }), test_blob_bytes)
        .await
        .into_response();

    let (_, location) = result.headers().iter().find(|(k, _)| k.as_str() == "location").unwrap();

    assert_eq!(result.status(), StatusCode::CREATED);
    assert!(location.to_str().unwrap().contains(NAMESPACE));
  }

  #[tokio::test]
  async fn test_get_blob() {
    let test_blob_bytes: Bytes = Bytes::from(TEST_BLOB_STRING);
    let client_digest = get_sha256_digest(&test_blob_bytes.to_vec());

    let result =
      get_blob(Path((NAMESPACE.to_string(), client_digest.clone()))).await.into_response();

    println!("headers: {:?}", result.headers());

    let (_, digest) =
      result.headers().iter().find(|(k, _)| k.as_str() == "docker-content-digest").unwrap();

    assert_eq!(result.status(), StatusCode::OK);

    // NOTE: The spec says "The Docker-Content-Digest header returns the
    //       canonical digest of the uploaded blob which MAY differ from the
    //       provided digest", but since we only support sha256 we can assume
    //       something is wrong if the digest doesn't match.
    assert_eq!(digest.to_str().unwrap(), client_digest);
  }
}
