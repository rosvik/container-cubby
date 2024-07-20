use crate::utils;
use axum::{
  extract::Request,
  http::{HeaderMap, HeaderValue, StatusCode},
  middleware::Next,
  response::Response,
};
use std::env;

pub async fn log_requests(
  headers: HeaderMap,
  request: Request,
  next: Next,
) -> Result<Response, StatusCode> {
  println!("\n\x1b[1;34m{} {:?}\x1b[0m", &request.method(), &request.uri());
  headers.iter().for_each(|(name, value)| {
    println!("  \x1b[36m{}: {:?}\x1b[0m", name, value);
  });
  Ok(next.run(request).await)
}

pub async fn basic_authenticate(headers: HeaderMap, request: Request, next: Next) -> Response {
  let auth_enabled = env::var("AUTH_ENABLED").unwrap_or_default().parse::<bool>().unwrap_or(true);
  if !auth_enabled {
    return next.run(request).await;
  }
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
