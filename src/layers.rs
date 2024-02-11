use axum::{
  extract::Request,
  http::{HeaderMap, StatusCode},
  middleware::Next,
  response::Response,
};

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
