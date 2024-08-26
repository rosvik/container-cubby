use crate::utils;
use actix_web::dev::ServiceRequest;
use axum::{
  extract::Request,
  http::{HeaderMap, HeaderValue, StatusCode},
  middleware::Next,
  response::Response,
};
use std::env;

pub fn log_requests(request: &ServiceRequest) {
  println!("\n\x1b[1;34m{} {:?}\x1b[0m", &request.method(), &request.uri());
  request.headers().iter().for_each(|(name, value)| {
    println!("  \x1b[36m{}: {:?}\x1b[0m", name, value);
  });
}

pub async fn basic_authenticate(headers: HeaderMap, request: Request, next: Next) -> Response {
  fn unauthorized() -> Response {
    Response::builder()
      .status(StatusCode::UNAUTHORIZED)
      .body(axum::body::Body::default())
      .unwrap_or_default()
  }
  match headers.get("Authorization") {
    None => {
      // If Authorization header is not set, issue a basic auth challenge
      Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header("WWW-Authenticate", HeaderValue::from_static("Basic realm=\"\", charset=\"UTF-8\""))
        .body(axum::body::Body::default())
        .unwrap_or_default()
    }
    Some(authorization) => {
      let auth_header_value = authorization.to_str().unwrap_or_default().to_string();
      let b64 = auth_header_value.split_whitespace().last().unwrap_or_default().to_string();
      let request_credentials = utils::decode_base64(b64).unwrap_or_default();
      if request_credentials.is_empty() {
        return unauthorized();
      }

      let username = match env::var("USERNAME") {
        Ok(username) => username,
        Err(_) => return unauthorized(),
      };
      let password = match env::var("PASSWORD") {
        Ok(password) => password,
        Err(_) => return unauthorized(),
      };
      let credentials = format!("{}:{}", username, password);

      if credentials != request_credentials {
        let request_username = request_credentials.split(':').next().unwrap();
        println!(
          "\x1b[1;31mFailed auth: Incorrect username/password provided for user '{request_username}'\x1b[0m"
        );
        return unauthorized();
      }

      next.run(request).await
    }
  }
}
