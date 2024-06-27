mod db;
mod digestor;
mod middleware;
mod utils;

use axum::{
  body::Bytes,
  extract::{DefaultBodyLimit, Path, Query},
  http::{HeaderMap, HeaderValue, StatusCode},
  response::IntoResponse,
  routing::{delete, get, patch, post, put},
  Router,
};
use dotenv::dotenv;
use serde::Deserialize;
use std::env;
use tower::ServiceBuilder;
use uuid::Uuid;

const PROTOCOL: &str = "http";
const HOST: &str = "0.0.0.0";
const DEFAULT_PORT: &str = "8602";
const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
  dotenv().ok();
  let port = env::var("PORT").unwrap_or(DEFAULT_PORT.to_string());
  let addr = format!("{}:{}", HOST, port);

  dotenv().ok();
  if env::var("USERNAME").is_err() || env::var("PASSWORD").is_err() {
    println!(
      "\x1b[1;33mINFO: Username/password was not provided. Registry is in read-only mode.\x1b[0m"
    );
  };

  db::init().unwrap();
  let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
  println!("Listening on \x1b[1;4m{PROTOCOL}://{addr}/\x1b[0m");
  axum::serve(listener, router()).await.unwrap();
}

fn router() -> Router {
  let unlimited_upload_size: DefaultBodyLimit = DefaultBodyLimit::disable();
  let basic_auth = axum::middleware::from_fn(middleware::basic_authenticate);
  let upload_middleware =
    ServiceBuilder::new().layer(basic_auth.clone()).layer(unlimited_upload_size.clone());
  Router::new()
    .route("/", get(|| async { format!("{CRATE_NAME} v{CRATE_VERSION}") }))
    .route("/v2/", get("Authenticated").layer(basic_auth.clone()))
    .route("/v2/:name/blobs/:digest", get(get_blob))
    .route("/v2/:name/manifests/:reference", get(get_manifest))
    .route("/v2/:name/blobs/uploads/", post(post_blob_upload).layer(upload_middleware.clone()))
    .route(
      "/v2/:name/blobs/uploads/:reference",
      put(put_blob_upload).layer(upload_middleware.clone()),
    )
    .route(
      "/v2/:name/blobs/uploads/:reference",
      patch(patch_blob_upload).layer(upload_middleware.clone()),
    )
    .route("/v2/:name/manifests/:reference", put(put_manifest).layer(upload_middleware.clone()))
    .route("/v2/:name/tags/list", get(get_tags_list))
    .route("/v2/:name/manifests/:reference", delete(delete_manifest).layer(basic_auth.clone()))
    .route("/v2/:name/blobs/:digest", delete(delete_blob).layer(basic_auth.clone()))
    .route("/v2/:name/blobs/uploads/:reference", get(get_blob_upload))
    .layer(axum::middleware::from_fn(middleware::log_requests))
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
      return (StatusCode::NOT_FOUND, HeaderMap::new(), "".into());
    }
  };

  let mut headers = HeaderMap::new();
  headers.insert(
    "Docker-Content-Digest",
    HeaderValue::from_str(digestor::get_sha256_digest(&manifest.data.clone().unwrap()).as_str())
      .unwrap(),
  );

  // In a successful response, the Content-Type header will indicate the type of
  // the returned manifest.
  headers.insert(
    "Content-Type",
    HeaderValue::from_str("application/vnd.oci.image.manifest.v1+json").unwrap(),
  );

  (StatusCode::OK, headers, Bytes::from(manifest.data.unwrap()))
}

#[derive(Deserialize)]
struct PostBlobParameters {
  digest: Option<String>,
}
/// end-4: `POST /v2/<name>/blobs/uploads/?digest=<digest>` => 201/202 / 404/400
///
/// REQUEST
/// - Content-Length: {length}          (must match the blob's actual content length)
/// - Content-Type: `application/octet-stream`
/// - Body: {blob byte stream}
///
/// RESPONSE
/// - Location: {blob-location}         (a pullable blob URL)
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#single-post
async fn post_blob_upload(
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
/// - Content-Type: `application/octet-stream`
/// - Content-Range: {range}   (byte range of the chunk, inclusive on both ends)
/// - Content-Length: {length}    (must match the chunk's actual content length)
/// - Body: {chunk byte stream}
///
/// RESPONSE
/// - Location: {location}                        (url to the next chunk upload)
/// - Range: 0-{end-of-range}          (0 to position of the last uploaded byte)
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-a-blob-in-chunks
async fn patch_blob_upload(
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
      // NOTE: This is a conformance error, but since clients doesn't always
      //       include it, we continue the flow.
      println!(
        "Warning: Not able to parse request content length from headers. Data is {} bytes.",
        data.len()
      );
      data.len()
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

  let mut headers = HeaderMap::new();
  let location = format!("/v2/{}/blobs/uploads/{}", name, reference);

  // Each successful chunk upload MUST have a 202 Accepted response code, and
  // MUST have the following headers:
  // - Location: <location>
  // - Range: 0-<end-of-range>
  headers.insert("Location", HeaderValue::from_str(&location).unwrap());
  headers.insert("Range", HeaderValue::from_str(format!("0-{}", range_end).as_str()).unwrap());

  (StatusCode::ACCEPTED, headers, ())
}

#[derive(Deserialize)]
struct PutBlobParameters {
  digest: String,
}
/// end-6: `PUT /v2/<name>/blobs/uploads/<reference>?digest=<digest>` => 201 / 404/400
///
/// REQUEST
/// - Content-Length: {length}         (must match blob or chunk content length)
/// - Content-Type: `application/octet-stream`
/// - Content-Range: {chunk range}     (if the blob is being uploaded in chunks)
/// - Body: {blob byte stream}
///
/// RESPONSE
/// - Location: {blob-location}                            (a pullable blob URL)
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put
async fn put_blob_upload(
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
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-manifests
async fn put_manifest(
  Path((name, reference)): Path<(String, String)>,
  data: Bytes,
) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  match db::insert_manifest(&conn, &name, &reference, data.to_vec()) {
    Ok(_) => {}
    Err(e) => {
      if e.sqlite_error_code() != Some(rusqlite::ErrorCode::ConstraintViolation) {
        return (StatusCode::INTERNAL_SERVER_ERROR, HeaderMap::new(), ());
      }
      // We have already stored this blob. Until the spec tells us what to do in
      // this case, we treat it as a success and continue the normal flow.
      println!("Warning: Duplicate manifest, name='{}' reference='{}'", name, reference);
    }
  };

  let mut headers = HeaderMap::new();
  headers.insert("Location", HeaderValue::from_str("/v2/{name}/manifests/{reference}").unwrap());

  (StatusCode::CREATED, headers, ())
}

/// end-8: `GET /v2/<name>/tags/list` => 200 / 404
///
/// skopeo list-tags docker://docker.io/rosvik/tiny-registry
/// {
///   "Repository": "docker.io/rosvik/tiny-registry",
///   "Tags": [
///       "v0.1"
///   ]
/// }
async fn get_tags_list(Path(name): Path<String>) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  let tags = match db::get_tags(&conn, &name) {
    Ok(tags) => tags,
    Err(e) => {
      println!("Error getting tags: {:?}", e);
      return (StatusCode::NOT_FOUND, HeaderMap::new(), "".into());
    }
  };

  let mut headers = HeaderMap::new();
  headers.insert("Content-Type", HeaderValue::from_str("application/json").unwrap());

  (StatusCode::OK, headers, serde_json::to_string(&tags).unwrap())
}

/// end-9: `DELETE /v2/<name>/manifests/<reference>` => 202 / 404
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#deleting-manifests
async fn delete_manifest(Path((name, reference)): Path<(String, String)>) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  match db::delete_manifest(&conn, &name, &reference) {
    Ok(num_rows_changed) => {
      // If the repository does not exist, the response MUST return 404 Not Found.
      if num_rows_changed == 0 {
        println!("Warning: Manifest not found, name='{}' reference='{}'", name, reference);
        return StatusCode::NOT_FOUND;
      }
    }
    Err(e) => {
      println!("Error deleting manifest: {:?}", e);
      return StatusCode::INTERNAL_SERVER_ERROR;
    }
  }

  // Upon success, the registry MUST respond with a 202 Accepted code.
  StatusCode::ACCEPTED
}

/// end-10: `DELETE /v2/<name>/blobs/<digest>` => 202 / 404
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#deleting-blobs
async fn delete_blob(Path((name, digest)): Path<(String, String)>) -> impl IntoResponse {
  let conn = db::connect().unwrap();
  match db::delete_blob(&conn, &name, &digest) {
    Ok(num_rows_changed) => {
      // If the blob is not found, a 404 Not Found code MUST be returned.
      if num_rows_changed == 0 {
        println!("Warning: Blob not found, name='{}' digest='{}'", name, digest);
        return StatusCode::NOT_FOUND;
      }
    }
    Err(e) => {
      println!("Error deleting blob: {:?}", e);
      return StatusCode::INTERNAL_SERVER_ERROR;
    }
  }

  // Upon success, the registry MUST respond with code 202 Accepted.
  StatusCode::ACCEPTED
}

/// end-13: `GET /v2/<name>/blobs/uploads/<reference>` => 200 / 404
///
/// RESPONSE:
/// - Location: {blob-location}                            (a pullable blob URL)
/// - Range: 0-{end-of-range}          (0 to position of the last uploaded byte)
///
/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-a-blob-in-chunks
async fn get_blob_upload(Path((name, reference)): Path<(String, String)>) -> impl IntoResponse {
  // To get the current status after a 416 error, issue a GET request to a URL
  // <location> (end-13). The following chunk upload SHOULD use the <location>
  // provided in the response.

  let conn = db::connect().unwrap();
  let hunk = match db::get_hunk(&conn, &name, &reference) {
    Ok(hunk) => hunk,
    Err(e) => {
      println!("Error getting hunk: {:?}", e);
      return (StatusCode::NOT_FOUND, HeaderMap::new());
    }
  };
  let mut headers = HeaderMap::new();

  // The <location> refers to the URL obtained from any preceding POST or PATCH
  // request.
  let location = format!("/v2/{}/blobs/uploads/{}", name, reference);
  headers.insert("Location", HeaderValue::from_str(location.as_str()).unwrap());

  // The <end-of-range> value is the position of the last uploaded byte.
  // NOTE: If the hunk is empty, end_of_range is set to -1 (`range: 0--1`). It's
  //       unclear what's the expected behavior in this case.
  let end_of_range: i64 = (hunk.data.unwrap_or(Vec::new()).len() as i64) - 1;
  headers.insert("Range", HeaderValue::from_str(format!("0-{}", end_of_range).as_str()).unwrap());

  // The response to an active upload <location> MUST be a 204 No Content
  // response code
  (StatusCode::NO_CONTENT, headers)
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

    let _ = db::init();
    let test_blob_bytes: Bytes = Bytes::from(test_blob_string);

    let digest = get_sha256_digest(&test_blob_bytes.to_vec());

    let result = post_blob_upload(
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
    let _ = db::init();

    // POST to get reference
    let response = post_blob_upload(
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
    let put_response = put_blob_upload(
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
    let _ = db::init();

    // POST to get reference
    let response = post_blob_upload(
      Path(namespace.clone()),
      Query(PostBlobParameters { digest: None }),
      Bytes::new(),
    )
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
      patch_blob_upload(Path((namespace.clone(), reference.to_string())), headers, chunk)
        .await
        .into_response();
    assert_eq!(patch_response.status(), StatusCode::ACCEPTED);

    // PATCH second chunk
    let chunk = Bytes::from("BBBB");
    let mut headers = HeaderMap::new();
    headers.insert("Content-Length", HeaderValue::from_str("4").unwrap());
    headers.insert("Content-Range", HeaderValue::from_str("4-7").unwrap());
    let patch_response =
      patch_blob_upload(Path((namespace.clone(), reference.to_string())), headers, chunk)
        .await
        .into_response();
    assert_eq!(patch_response.status(), StatusCode::ACCEPTED);

    // PUT blob
    let mut headers = HeaderMap::new();
    headers.insert("Content-Length", HeaderValue::from_str("0").unwrap());
    let digest = get_sha256_digest(&"AAAABBBB".as_bytes().to_vec());
    let result = put_blob_upload(
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
    let _ = db::init();
    let test_blob_bytes: Bytes = Bytes::from(test_blob_string);
    let client_digest = get_sha256_digest(&test_blob_bytes.to_vec());

    {
      // First, POST the blob
      post_blob_upload(
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
