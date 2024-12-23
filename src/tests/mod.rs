mod utils;

use std::fs::File;

use super::*;
use crate::utils::encode_base64;
use actix_web::{
  dev::Service,
  http::{Method, StatusCode},
  test, App,
};
use middleware::basic_auth::BasicAuth;
use storage::get_xattr_media_type;
use utils::*;

#[test]
async fn test_create_blob() {
  let name: String = get_random_namespace();
  let blob = "testblob".as_bytes();
  let digest = digestor::get_sha256_digest(&blob.to_vec());

  let app =
    App::new().service(web::resource("/v2/{name:[^{}]+}/blobs/uploads/").post(post_blob_upload));
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
  let name: String = get_random_namespace();
  let blob = "testblob".as_bytes();
  let digest = digestor::get_sha256_digest(&blob.to_vec());

  let app = App::new()
    .service(web::resource("/v2/{name:[^{}]+}/blobs/uploads/").post(post_blob_upload))
    .service(web::resource("/v2/{name:[^{}]+}/blobs/uploads/{reference}").put(put_blob_upload));
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
  let name: String = get_random_namespace();
  let blob = "testblob".as_bytes();
  let digest = digestor::get_sha256_digest(&blob.to_vec());

  let app = App::new()
    .service(web::resource("/v2/{name:[^{}]+}/blobs/uploads/").post(post_blob_upload))
    .service(web::resource("/v2/{name:[^{}]+}/blobs/{digest}").get(get_blob));
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
  let name: String = get_random_namespace();
  let manifest = include_str!("./fixtures/image_manifest.json");

  let app =
    App::new().service(web::resource("/v2/{name:[^{}]+}/manifests/{reference}").put(put_manifest));
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

#[test]
async fn test_get_manifest() {
  let name: String = get_random_namespace();
  let manifest = include_str!("./fixtures/image_manifest.json");
  let manifest_source = serde_json::from_str::<schemas::ImageManifest>(manifest).unwrap();

  let app =
    App::new().service(web::resource("/v2/{name:[^{}]+}/manifests/{reference}").put(put_manifest));
  let service = test::init_service(app).await;

  // PUT manifest
  let uri = format!("/v2/{}/manifests/latest", name);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload(manifest)
    .insert_header(("Content-Type", "application/vnd.docker.distribution.manifest.v2+json"))
    .method(Method::PUT)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::CREATED);

  // GET manifest
  // TODO: Figure out how to use the same service for PUT and GET when the path
  //       is the same.
  let app =
    App::new().service(web::resource("/v2/{name:[^{}]+}/manifests/{reference}").get(get_manifest));
  let service = test::init_service(app).await;

  let uri = format!("/v2/{}/manifests/latest", name);
  let req = test::TestRequest::with_uri(uri.as_str())
    .insert_header(("Accept", "application/vnd.docker.distribution.manifest.v2+json"))
    .method(Method::GET)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::OK);

  let bytes = test::read_body(res).await;
  let manifest_result = serde_json::from_slice::<schemas::ImageManifest>(&bytes).unwrap();
  assert_eq!(manifest_result.config.digest, manifest_source.config.digest);

  // GET manifest based on its digest
  let manifest_digest = "sha256:edee272db7445c0aedfa7892df3f734fa6117221e37389063e65648ba47f7b00";
  let uri = format!("/v2/{}/manifests/{}", name, manifest_digest);
  let req = test::TestRequest::with_uri(uri.as_str())
    .insert_header(("Accept", "application/vnd.docker.distribution.manifest.v2+json"))
    .method(Method::GET)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::OK);

  // DELETE manifest based on its digest
  let app = App::new()
    .service(web::resource("/v2/{name:[^{}]+}/manifests/{reference}").delete(delete_manifest));
  let service = test::init_service(app).await;

  let uri = format!("/v2/{}/manifests/{}", name, manifest_digest);
  let req = test::TestRequest::with_uri(uri.as_str())
    .insert_header(("Accept", "application/vnd.docker.distribution.manifest.v2+json"))
    .method(Method::DELETE)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::ACCEPTED);
}

#[test]
async fn test_manifest_media_type() {
  let manifest = include_str!("./fixtures/image_manifest_no_media_type.json");
  let manifest_digest = digestor::get_sha256_digest(&manifest.as_bytes().to_vec());
  let name: String = get_random_namespace();

  let app =
    App::new().service(web::resource("/v2/{name:[^{}]+}/manifests/{reference}").put(put_manifest));
  let service = test::init_service(app).await;

  // PUT manifest
  let uri = format!("/v2/{}/manifests/latest", name);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload(manifest)
    .insert_header(("Content-Type", "application/vnd.docker.distribution.manifest.v2+json"))
    .method(Method::PUT)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::CREATED);

  // GET manifest
  let app =
    App::new().service(web::resource("/v2/{name:[^{}]+}/manifests/{reference}").get(get_manifest));
  let service = test::init_service(app).await;

  // Based on tag
  let uri = format!("/v2/{}/manifests/latest", name);
  let req = test::TestRequest::with_uri(uri.as_str())
    .insert_header(("Accept", "application/vnd.docker.distribution.manifest.v2+json"))
    .method(Method::GET)
    .to_request();
  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::OK);
  let response_media_type = res.headers().get("Content-Type").unwrap();
  assert_eq!(
    response_media_type.to_str().unwrap(),
    "application/vnd.docker.distribution.manifest.v2+json"
  );

  // Based on digest
  let uri = format!("/v2/{}/manifests/{}", name, manifest_digest);
  let req = test::TestRequest::with_uri(uri.as_str())
    .insert_header(("Accept", "application/vnd.docker.distribution.manifest.v2+json"))
    .method(Method::GET)
    .to_request();
  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::OK);
  let response_media_type = res.headers().get("Content-Type").unwrap();
  assert_eq!(
    response_media_type.to_str().unwrap(),
    "application/vnd.docker.distribution.manifest.v2+json"
  );
}

#[test]
async fn test_push_as_hunks() {
  let name: String = get_random_namespace();
  let blob = "AAAABBBBCCCC".as_bytes();
  let digest = digestor::get_sha256_digest(&blob.to_vec());

  let app = App::new()
    .service(web::resource("/v2/{name:[^{}]+}/blobs/uploads/").post(post_blob_upload))
    .service(web::resource("/v2/{name:[^{}]+}/blobs/uploads/{reference}").patch(patch_blob_upload));
  let service = test::init_service(app).await;

  // POST to get reference
  let uri = format!("/v2/{}/blobs/uploads/", name);
  let req = test::TestRequest::with_uri(uri.as_str())
    .insert_header(("Content-Length", blob.len().to_string()))
    .insert_header(("Content-Type", "application/octet-stream"))
    .insert_header(("Content-Range", "0-3"))
    .method(Method::POST)
    .to_request();
  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::ACCEPTED);
  let location = res.headers().get("Location").unwrap();
  let reference = location.to_str().unwrap().split('/').last().unwrap();

  // PATCH first chunk
  let uri = format!("/v2/{}/blobs/uploads/{}", name, reference);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload("AAAA")
    .insert_header(("Content-Length", "4"))
    .insert_header(("Content-Range", "0-3"))
    .method(Method::PATCH)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::ACCEPTED);

  // PATCH second chunk
  let uri = format!("/v2/{}/blobs/uploads/{}", name, reference);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload("BBBB")
    .insert_header(("Content-Length", "4"))
    .insert_header(("Content-Range", "4-7"))
    .method(Method::PATCH)
    .to_request();

  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::ACCEPTED);

  // PUT blob
  // TODO: Figure out how to use the same service for PUT and GET when the path
  //       is the same.
  let app = App::new()
    .service(web::resource("/v2/{name:[^{}]+}/blobs/uploads/{reference}").put(put_blob_upload))
    .service(web::resource("/v2/{name:[^{}]+}/blobs/{digest}").get(get_blob));
  let service = test::init_service(app).await;

  let uri = format!("/v2/{}/blobs/uploads/{}?digest={}", name, reference, digest);
  let req = test::TestRequest::with_uri(uri.as_str())
    .set_payload("CCCC")
    .insert_header(("Content-Length", "4"))
    .method(Method::PUT)
    .to_request();
  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::CREATED);

  // Verify that the blob can be retrieved
  let uri = format!("/v2/{}/blobs/{}", name, digest);
  let req = test::TestRequest::with_uri(uri.as_str()).method(Method::GET).to_request();
  let res = service.call(req).await.unwrap();
  assert_eq!(res.status(), StatusCode::OK);

  let bytes = test::read_body(res).await;
  assert_eq!(bytes, blob);
}

#[test]
async fn test_basic_auth() {
  std::env::set_var("USERNAME", "admin");
  std::env::set_var("PASSWORD", "hunter2");
  let user = std::env::var("USERNAME").unwrap();
  let pass = std::env::var("PASSWORD").unwrap();

  let app =
    App::new().service(web::resource("/v2/").get(|| async { "Authenticated" })).wrap(BasicAuth);
  let service = test::init_service(app).await;

  // Auth challenge
  let req = test::TestRequest::with_uri("/v2/").method(Method::GET).to_request();
  let res = service.call(req).await;
  assert_eq!(res.unwrap().status(), StatusCode::UNAUTHORIZED);

  // Auth success
  let req = test::TestRequest::with_uri("/v2/")
    .insert_header((
      "Authorization",
      format!("Basic {}", encode_base64(format!("{}:{}", user, pass))),
    ))
    .method(Method::GET)
    .to_request();

  let res = service.call(req).await;
  assert_eq!(res.unwrap().status(), StatusCode::OK);
}

#[test]
async fn test_no_auth() {
  let app = App::new()
    .service(web::resource("/v2/").get(|| async { "Authenticated" }).wrap(BasicAuth))
    .service(web::resource("/v2/{name:[^{}]+}/manifests/{reference}").get(get_manifest));
  let service = test::init_service(app).await;

  // GET /v2/ should require authentication
  let req = test::TestRequest::with_uri("/v2/").to_request();
  let res = service.call(req).await;
  assert_eq!(res.unwrap().status(), StatusCode::UNAUTHORIZED);

  // GET /v2/test/manifests/latest should not require authentication
  let req = test::TestRequest::with_uri("/v2/test/manifests/latest").to_request();
  let res = service.call(req).await;
  // StatisCode::NOT_FOUND means authentication was not required
  assert_eq!(res.unwrap().status(), StatusCode::NOT_FOUND);
}

#[test]
async fn test_xattr() {
  let file = File::create("test.txt").unwrap();
  set_xattr_media_type(file, "application/vnd.docker.distribution.manifest.v2+json").unwrap();
  let file = File::open("test.txt").unwrap();
  let media_type = get_xattr_media_type(file).unwrap();
  assert_eq!(media_type, "application/vnd.docker.distribution.manifest.v2+json");
}
