mod db;
mod digestor;
mod utils;

use axum::{
  body::Bytes,
  extract::{Path, Query},
  http::{HeaderMap, HeaderValue, StatusCode},
  response::IntoResponse,
  routing::{get, patch, post, put},
  Router,
};
use serde::Deserialize;
use uuid::Uuid;

const HOST: &str = "0.0.0.0:8602";
const PROTOCOL: &str = "http";
const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
  db::init().unwrap();
  let listener = tokio::net::TcpListener::bind(HOST).await.unwrap();
  println!("Listening on {PROTOCOL}://{HOST}/");
  axum::serve(listener, router()).await.unwrap();
}

fn router() -> Router {
  Router::new()
    .route("/", get(|| async { format!("{CRATE_NAME} v{CRATE_VERSION}") }))
    .route("/v2", get(()))
    .route("/v2/:name/blobs/:digest", get(get_blob))
    .route("/v2/:name/manifests/:reference", get(get_manifest))
    .route("/v2/:name/blobs/uploads/", post(post_blob))
    .route("/v2/:name/blobs/uploads/:reference", put(put_blob))
    .route("/v2/:name/blobs/uploads/:reference", patch(patch_blob))
    .route("/v2/:name/manifests/:reference", put(put_manifest))
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
/// - Accept: {content type}                 (see spec / content-negotiation.md)
///
/// RESPONSE:
/// - Content-Type: {content type}           (see spec / content-negotiation.md)
/// - Docker-Content-Digest: {digest}    (canonical digest of the uploaded blob)
/// - Body: {manifest}
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests
async fn get_manifest(Path((name, reference)): Path<(String, String)>) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  let manifest = match db::get_manifest(&conn, &name, &reference) {
    Ok(manifest) => manifest,
    Err(e) => {
      println!("Error getting manifest: {:?}", e);
      return (StatusCode::NOT_FOUND, HeaderMap::new(), ());
    }
  };

  let mut headers = HeaderMap::new();
  headers.insert(
    "Docker-Content-Digest",
    HeaderValue::from_str(digestor::get_sha256_digest(&manifest.data.unwrap()).as_str()).unwrap(),
  );

  (StatusCode::OK, headers, ())
}

#[derive(Deserialize)]
struct PostBlobParameters {
  digest: Option<String>,
}
/// end-4: `POST /v2/<name>/blobs/uploads/?digest=<digest>` => 201/202 / 404/400
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
async fn post_blob(
  Path(name): Path<String>,
  Query(query): Query<PostBlobParameters>,
  data: Bytes,
) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  let digest = match query.digest {
    Some(digest) => digest, // end-4b
    None => {
      // end-4a

      // If the digest parameter is not provided, we are in the "POST then PUT"
      // or chunked upload flow (PATCH). We return the `location` header, which
      // points to an endpoint that accepts PUT <location>?digest=<digest>
      // (end-6) and PATCH <location> (end-5).
      // https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put
      // https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-a-blob-in-chunks

      // MUST contain a UUID representing a unique session ID
      let reference = Uuid::new_v4().to_string();
      db::insert_empty_hunk(&conn, &name, &reference).unwrap();

      let mut headers = HeaderMap::new();
      let location = format!("/v2/{}/blobs/uploads/{}", name, reference);
      headers.insert("Location", HeaderValue::from_str(location.as_str()).unwrap());

      // Upon success, the response MUST have a code of 202 Accepted
      return (StatusCode::ACCEPTED, headers, ());
    }
  };

  match db::verify_and_insert_blob(&conn, name.as_str(), digest.as_str(), &data) {
    Ok(_) => (),
    Err(e) => {
      if e == rusqlite::Error::InvalidQuery {
        // If the request is invalid, such as a <digest> with an invalid syntax,
        // a 400 Bad Request MUST be returned.
        return (StatusCode::BAD_REQUEST, HeaderMap::new(), ());
      }
      println!("Error inserting blob: {:?}", e);
      return (StatusCode::INTERNAL_SERVER_ERROR, HeaderMap::new(), ());
    }
  }

  let mut headers = HeaderMap::new();
  utils::insert_blob_location_header(&mut headers, name.as_str(), digest.as_str());

  // Successful completion of the request MUST return a 201 Created status code.
  (StatusCode::CREATED, headers, ())
}

/// end-5: `PATCH /v2/<name>/blobs/uploads/<reference>` => 202 / 404/416
///
/// NOTE: This should be referred to from a preceeding POST request to end-4a:
///
///  > For information on obtaining a session ID, reference the above
///    section on pushing a blob monolithically via the POST/PUT method.
///    The process remains unchanged for chunked upload, except that the
///    post request MUST include the following header:
///
/// REQUEST
/// - Content-Type: "application/octet-stream"
/// - Content-Range: {range}   (byte range of the chunk, inclusive on both ends)
/// - Content-Length: {length}    (must match the chunk's actual content length)
/// - Body: {chunk byte stream}
///
/// RESPONSE
/// - Location: {location}                        (url to the next chunk upload)
/// - Range: 0-{end-of-range}          (0 to position of the last uploaded byte)
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-a-blob-in-chunks
async fn patch_blob(
  Path((name, reference)): Path<(String, String)>,
  headers: HeaderMap,
  data: Bytes,
) -> impl IntoResponse {
  let (range, range_start, range_end) = match utils::get_content_range(&headers) {
    Some(range) => range,
    None => {
      if headers.get("Content-Range").is_some() {
        println!("Error: Invalid range: {:?}", headers);
        return (StatusCode::BAD_REQUEST, HeaderMap::new(), ());
      }

      // NOTE: The spec isn't clear about the case where the Content-Range
      // header is missing. But since the conformance tests and tools like
      // podman excludes it when the entire blob is being uploaded
      // monolithically, we'll assume that no range means the entire blob.
      // https://github.com/opencontainers/distribution-spec/issues/506
      let range_start = 0;
      let range_end = data.len() - 1;
      (format!("{}-{}", range_start, range_end), range_start, range_end)
    }
  };
  let req_length = match utils::get_content_length(&headers) {
    Some(length) => length,
    None => {
      println!("Error: Invalid content length: {:?}", headers);
      return (StatusCode::BAD_REQUEST, HeaderMap::new(), ());
    }
  };

  // Content-Length header MUST match the actual number of bytes in the chunk.
  if req_length != data.len() {
    println!("Error: Invalid content length: Content-Length={}, data={}", req_length, data.len());
    return (StatusCode::BAD_REQUEST, HeaderMap::new(), ());
  }

  let conn = db::connect().unwrap();
  {
    let stored_hunk = match db::get_hunk(&conn, &name, &reference) {
      Ok(hunk) => hunk,
      Err(e) => {
        println!("Error: Could not get hunk: {:?}", e);
        return (StatusCode::NOT_FOUND, HeaderMap::new(), ());
      }
    };

    if stored_hunk.last_byte.is_none() && range_start != 0 {
      // The first chunk's range MUST begin with 0.
      println!("Error: First chunk's range must begin with 0: {:?}", range);
      return (StatusCode::RANGE_NOT_SATISFIABLE, HeaderMap::new(), ());
    } else if stored_hunk.last_byte.is_some() && stored_hunk.last_byte.unwrap() + 1 != range_start {
      // Chunks MUST be uploaded in order, with the first byte of a chunk being
      // the last chunk's <end-of-range> plus one. If a chunk is uploaded out of
      // order, the registry MUST respond with a 416 Requested Range Not
      // Satisfiable code.
      println!(
        "Error: Uploaded hunk range did not match stored hunk: Stored: {}, req_first_byte: {}",
        stored_hunk.last_byte.unwrap(),
        range_start
      );
      return (StatusCode::RANGE_NOT_SATISFIABLE, HeaderMap::new(), ());
    }

    if range_start + req_length != range_end + 1 {
      // The Content-Range header MUST specify the range of bytes being uploaded
      // in the format `0-{end-of-range}`.
      println!("Error: Invalid range: {} ({range_start}+{req_length}!={range_end}+1)", range);
      return (StatusCode::BAD_REQUEST, HeaderMap::new(), ());
    }
  }

  db::append_hunk(&conn, &name, &reference, data.to_vec()).unwrap();
  (StatusCode::ACCEPTED, HeaderMap::new(), ())
}

#[derive(Deserialize)]
struct PutBlobParameters {
  digest: String,
}
/// end-6: `PUT /v2/<name>/blobs/uploads/<reference>?digest=<digest>` => 201 / 404/400
///
/// REQUEST
/// - Content-Length: {length}         (must match blob or chunk content length)
/// - Content-Type: "application/octet-stream"
/// - Content-Range: {chunk range}     (if the blob is being uploaded in chunks)
/// - Body: {blob byte stream}
///
/// RESPONSE
/// - Location: {blob-location}                            (a pullable blob URL)
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put
async fn put_blob(
  Path((name, reference)): Path<(String, String)>,
  Query(query): Query<PutBlobParameters>,
  headers: HeaderMap,
  data: Bytes,
) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  let digest = query.digest;
  let req_length = utils::get_content_length(&headers).unwrap_or(0);

  // Content-Length header MUST match the actual number of bytes in the chunk.
  if req_length != data.len() {
    println!("Error: Invalid content length: Content-Length={}, data={}", req_length, data.len());
    return (StatusCode::BAD_REQUEST, HeaderMap::new(), ());
  }

  if !data.is_empty() {
    // TODO: Verify Content-Range

    // We have recieved the final hunk of a blob or the entire blob in one go
    db::append_hunk(&conn, &name, &reference, data.to_vec()).unwrap();
  }

  match db::commit_hunk(&conn, name.as_str(), &reference, digest.as_str()) {
    Ok(_) => (),
    Err(e) => {
      if e == rusqlite::Error::InvalidQuery {
        // If the request is invalid, such as a <digest> with an invalid syntax,
        // a 400 Bad Request MUST be returned.
        return (StatusCode::BAD_REQUEST, HeaderMap::new(), ());
      }
      println!("Error inserting blob: {:?}", e);
      return (StatusCode::INTERNAL_SERVER_ERROR, HeaderMap::new(), ());
    }
  }

  let mut headers = HeaderMap::new();
  utils::insert_blob_location_header(&mut headers, "name", "digest");

  (StatusCode::CREATED, headers, ())
}

/// end-7: `PUT /v2/<name>/manifests/<reference>` => 201 / 404
///
/// REQUEST
/// - Content-Type: {content type}           (same as mediaType in the manifest)
/// - Body: {manifest byte stream}
///
/// RESPONSE
/// - Location: {manifest-location}                    (a pullable manifest URL)
async fn put_manifest(
  Path((name, reference)): Path<(String, String)>,
  data: Bytes,
) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  db::insert_manifest(&conn, &name, &reference, data.to_vec()).unwrap();

  let mut headers = HeaderMap::new();
  headers.insert("Location", HeaderValue::from_str("/v2/{name}/manifests/{reference}").unwrap());

  (StatusCode::CREATED, headers, ())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::digestor::get_sha256_digest;
  use std::iter::repeat_with;

  fn get_random_namespace() -> String {
    let random_string: String = repeat_with(fastrand::alphanumeric).take(10).collect();
    format!("test:{CRATE_VERSION}:{}", random_string)
  }

  #[tokio::test]
  async fn test_post_blob() {
    let namespace: String = get_random_namespace();
    let test_blob_string: &str = "testblob";

    db::init().unwrap();
    let test_blob_bytes: Bytes = Bytes::from(test_blob_string);

    let digest = get_sha256_digest(&test_blob_bytes.to_vec());

    let result = post_blob(
      Path(namespace.to_string()),
      Query(PostBlobParameters {
        digest: Some(digest),
      }),
      test_blob_bytes,
    )
    .await
    .into_response();

    let location = result.headers().get("Location").unwrap();

    assert_eq!(result.status(), StatusCode::CREATED);
    assert!(location.to_str().unwrap().contains(namespace.as_str()));
  }

  #[tokio::test]
  async fn test_post_then_put() {
    let namespace: String = get_random_namespace();
    let test_blob: Bytes = Bytes::from("test_post_then_put");
    db::init().unwrap();

    // POST to get reference
    let response = post_blob(
      Path(namespace.to_string()),
      Query(PostBlobParameters { digest: None }),
      Bytes::new(),
    )
    .await
    .into_response();
    let location = response.headers().get("Location").unwrap();
    let reference = location.to_str().unwrap().split('/').last().unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(!reference.is_empty());

    // PUT entire blob
    let digest = get_sha256_digest(&test_blob.to_vec());
    let mut headers = HeaderMap::new();
    headers.insert(
      "Content-Length",
      HeaderValue::from_str(test_blob.len().to_string().as_str()).unwrap(),
    );
    let put_response = put_blob(
      Path((namespace.to_string(), reference.to_string())),
      Query(PutBlobParameters {
        digest: digest.clone(),
      }),
      headers,
      test_blob,
    )
    .await
    .into_response();
    assert_eq!(put_response.status(), StatusCode::CREATED);

    // Verify that the blob can be retrieved
    let blob = get_blob(Path((namespace.to_string(), digest))).await.into_response();
    assert_eq!(blob.status(), StatusCode::OK);
  }

  #[tokio::test]
  async fn test_push_as_hunks() {
    let namespace: String = get_random_namespace();
    db::init().unwrap();

    // POST to get reference
    let response =
      post_blob(Path(namespace.clone()), Query(PostBlobParameters { digest: None }), Bytes::new())
        .await
        .into_response();
    let location = response.headers().get("Location").unwrap();
    let reference = location.to_str().unwrap().split('/').last().unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(!reference.is_empty());

    // PATCH first chunk
    let chunk = Bytes::from("AAAA");
    let mut headers = HeaderMap::new();
    headers.insert("Content-Length", HeaderValue::from_str("4").unwrap());
    headers.insert("Content-Range", HeaderValue::from_str("0-3").unwrap());
    let patch_response =
      patch_blob(Path((namespace.clone(), reference.to_string())), headers, chunk)
        .await
        .into_response();
    assert_eq!(patch_response.status(), StatusCode::ACCEPTED);

    // PATCH second chunk
    let chunk = Bytes::from("BBBB");
    let mut headers = HeaderMap::new();
    headers.insert("Content-Length", HeaderValue::from_str("4").unwrap());
    headers.insert("Content-Range", HeaderValue::from_str("4-7").unwrap());
    let patch_response =
      patch_blob(Path((namespace.clone(), reference.to_string())), headers, chunk)
        .await
        .into_response();
    assert_eq!(patch_response.status(), StatusCode::ACCEPTED);

    // PUT blob
    let mut headers = HeaderMap::new();
    headers.insert("Content-Length", HeaderValue::from_str("0").unwrap());
    let digest = get_sha256_digest(&"AAAABBBB".as_bytes().to_vec());
    let result = put_blob(
      Path((namespace.clone(), reference.to_string())),
      Query(PutBlobParameters {
        digest: digest.clone(),
      }),
      headers,
      Bytes::new(),
    )
    .await
    .into_response();
    assert_eq!(result.status(), StatusCode::CREATED);

    // Verify that the blob can be retrieved
    let blob = get_blob(Path((namespace.clone(), digest.clone()))).await.into_response();
    assert_eq!(blob.status(), StatusCode::OK);
  }

  #[tokio::test]
  async fn test_get_blob() {
    let namespace = get_random_namespace();
    let test_blob_string: &str = "testblob";
    db::init().unwrap();
    let test_blob_bytes: Bytes = Bytes::from(test_blob_string);
    let client_digest = get_sha256_digest(&test_blob_bytes.to_vec());

    {
      // First, POST the blob
      post_blob(
        Path(namespace.to_string()),
        Query(PostBlobParameters {
          digest: Some(client_digest.clone()),
        }),
        test_blob_bytes,
      )
      .await;
    }

    let result =
      get_blob(Path((namespace.to_string(), client_digest.clone()))).await.into_response();
    let digest = result.headers().get("Docker-Content-Digest").unwrap();

    assert_eq!(result.status(), StatusCode::OK);

    // NOTE: The spec says "The Docker-Content-Digest header returns the
    //       canonical digest of the uploaded blob which MAY differ from the
    //       provided digest", but since we only support sha256 we can assume
    //       something is wrong if the digests don't match.
    assert_eq!(digest.to_str().unwrap(), client_digest);
  }
}
