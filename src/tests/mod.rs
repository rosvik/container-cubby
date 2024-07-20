mod utils;

use super::*;
use crate::digestor::get_sha256_digest;
use utils::get_random_namespace;

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
  headers
    .insert("Content-Length", HeaderValue::from_str(test_blob.len().to_string().as_str()).unwrap());
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

  let result = get_blob(Path((namespace.to_string(), client_digest.clone()))).await.into_response();
  let digest = result.headers().get("Docker-Content-Digest").unwrap();

  assert_eq!(result.status(), StatusCode::OK);

  // NOTE: The spec says "The Docker-Content-Digest header returns the
  //       canonical digest of the uploaded blob which MAY differ from the
  //       provided digest", but since we only support sha256 we can assume
  //       something is wrong if the digests don't match.
  assert_eq!(digest.to_str().unwrap(), client_digest);
}
