mod utils;

use super::*;
use actix_web::{
  dev::Service,
  http::{Method, StatusCode},
  test, App,
};
use utils::*;

#[test]
async fn test_create_blob() {
  let _ = db::init();

  let name: String = get_random_namespace();
  let blob = "testblob".as_bytes();
  let digest = digestor::get_sha256_digest(&blob.to_vec());

  let app = App::new().service(web::resource("/v2/{name}/blobs/uploads/").post(post_blob_upload));
  let service = test::init_service(app).await;

  let uri = format!("/v2/{}/blobs/uploads/?digest={}", name, digest);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload(blob)
    .insert_header(("Content-Length", blob.len().to_string()))
    .insert_header(("Content-Type", "application/octet-stream"))
    .insert_header(("Content-Range", "0-9"))
    .method(Method::POST)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::CREATED);

  let location = res.headers().get("Location").unwrap();
  assert!(location.to_str().unwrap().contains(name.as_str()));
}

#[test]
async fn test_post_then_put() {
  let _ = db::init();

  let name: String = get_random_namespace();
  let blob = "testblob".as_bytes();
  let digest = digestor::get_sha256_digest(&blob.to_vec());

  let app = App::new()
    .service(web::resource("/v2/{name}/blobs/uploads/").post(post_blob_upload))
    .service(web::resource("/v2/{name}/blobs/uploads/{reference}").put(put_blob_upload));
  let service = test::init_service(app).await;

  // POST to get reference
  let uri = format!("/v2/{}/blobs/uploads/", name);
  let req = test::TestRequest::with_uri(uri.as_str())
    .insert_header(("Content-Length", blob.len().to_string()))
    .insert_header(("Content-Type", "application/octet-stream"))
    .insert_header(("Content-Range", "0-9"))
    .method(Method::POST)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::ACCEPTED);

  let location = res.headers().get("Location").unwrap();
  let reference = location.to_str().unwrap().split('/').last().unwrap();

  // PUT entire blob
  let uri = format!("/v2/{}/blobs/uploads/{}?digest={}", name, reference, digest);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload(blob)
    .insert_header(("Content-Length", blob.len().to_string()))
    .insert_header(("Content-Type", "application/octet-stream"))
    .insert_header(("Content-Range", "0-9"))
    .method(Method::PUT)
    .to_request();
  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::CREATED);
}

#[test]
async fn test_get_blob() {
  let _ = db::init();

  let name: String = get_random_namespace();
  let blob = "testblob".as_bytes();
  let digest = digestor::get_sha256_digest(&blob.to_vec());

  let app = App::new()
    .service(web::resource("/v2/{name}/blobs/uploads/").post(post_blob_upload))
    .service(web::resource("/v2/{name}/blobs/{digest}").get(get_blob));
  let service = test::init_service(app).await;

  // POST to get reference
  let uri = format!("/v2/{}/blobs/uploads/?digest={}", name, digest);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload(blob)
    .insert_header(("Content-Length", blob.len().to_string()))
    .insert_header(("Content-Type", "application/octet-stream"))
    .insert_header(("Content-Range", "0-9"))
    .method(Method::POST)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::CREATED);

  // GET blob
  let uri = format!("/v2/{}/blobs/{}", name, digest);
  let req = test::TestRequest::with_uri(uri.as_str()).method(Method::GET).to_request();
  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::OK);

  // NOTE: The spec says "The Docker-Content-Digest header returns the canonical
  //       digest of the uploaded blob which MAY differ from the provided
  //       digest", but since we only support sha256 we can assume something is
  //       wrong if the digests don't match.
  let response_digest = res.headers().get("Docker-Content-Digest").unwrap();
  assert_eq!(response_digest.to_str().unwrap(), digest);

  let bytes = test::read_body(res).await;
  assert_eq!(bytes, blob);
}

#[test]
async fn test_put_manifest() {
  let _ = db::init();

  let name: String = get_random_namespace();
  let manifest = include_str!("./fixtures/manifest.json");

  let app = App::new().service(web::resource("/v2/{name}/manifests/{reference}").put(put_manifest));
  let service = test::init_service(app).await;

  let uri = format!("/v2/{}/manifests/latest", name);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload(manifest)
    .insert_header(("Content-Type", "application/vnd.docker.distribution.manifest.v2+json"))
    .method(Method::PUT)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::CREATED);

  let incomplete = "{\"schemaVersion\": 2}";
  let uri = format!("/v2/{}/manifests/incomplete", name);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload(incomplete)
    .insert_header(("Content-Type", "application/vnd.docker.distribution.manifest.v2+json"))
    .method(Method::PUT)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
