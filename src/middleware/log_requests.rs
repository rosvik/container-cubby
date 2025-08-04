use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
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
  println!("\n\x1b[1;35mRequest: \x1b[1;34m{} {:?}\x1b[0m", &req.method(), &req.uri());
  req.headers().iter().for_each(|(name, value)| {
    println!("  \x1b[36m{name}: {value:?}\x1b[0m");
  });
}

fn log_response<B>(res: &ServiceResponse<B>) {
  println!("\x1b[1;35mResponse: \x1b[34m{:?}\x1b[0m", res.status());
  res.headers().iter().for_each(|(name, value)| {
    println!("  \x1b[36m{name}: {value:?}\x1b[0m");
  });
}
