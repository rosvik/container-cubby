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
