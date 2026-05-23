use crate::utils::ansi::{BLUE, CYAN, GREEN, MAGENTA, RED, RESET, YELLOW};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::StatusCode;
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

pub struct LogRequests;
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for LogRequests
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<B>;
  type Error = actix_web::Error;
  type InitError = ();
  type Transform = LogRequestsMiddleware<S>;
  type Future = Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    ready(Ok(LogRequestsMiddleware { service }))
  }
}

pub struct LogRequestsMiddleware<S> {
  service: S,
}
impl<S, B> Service<ServiceRequest> for LogRequestsMiddleware<S>
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<B>;
  type Error = actix_web::Error;
  type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

  fn call(&self, req: ServiceRequest) -> Self::Future {
    log_request(&req);

    let fut = self.service.call(req);
    Box::pin(async move {
      let res = fut.await?;
      log_response(&res);
      Ok(res)
    })
  }

  fn poll_ready(
    &self,
    ctx: &mut core::task::Context<'_>,
  ) -> std::task::Poll<Result<(), Self::Error>> {
    self.service.poll_ready(ctx)
  }
}

fn log_request(req: &ServiceRequest) {
  println!("\n{MAGENTA}Request: {BLUE}{} {:?}{RESET}", &req.method(), &req.uri());
  req.headers().iter().for_each(|(name, value)| {
    let value = match name.as_str().to_lowercase().as_str() {
      "authorization" => "*******",
      _ => value.to_str().unwrap_or_default(),
    };
    println!("  {CYAN}{name}: {value}{RESET}");
  });
}

fn log_response<B>(res: &ServiceResponse<B>) {
  let status_color = match res.status() {
    StatusCode::OK => GREEN,            // 200
    StatusCode::CREATED => GREEN,       // 201
    StatusCode::ACCEPTED => GREEN,      // 202
    StatusCode::NO_CONTENT => GREEN,    // 204
    StatusCode::UNAUTHORIZED => YELLOW, // 401
    StatusCode::NOT_FOUND => YELLOW,    // 404
    _ => RED,
  };

  println!("\n{MAGENTA}Response: {status_color}{:?}{RESET}", res.status());
  res.headers().iter().for_each(|(name, value)| {
    println!("  {CYAN}{name}: {value:?}{RESET}");
  });
}
