use crate::utils::decode_base64;
use actix_web::{
  body::EitherBody,
  dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
  http::header::HeaderValue,
  Error, HttpResponse,
};
use futures_util::{future::LocalBoxFuture, FutureExt};
use std::{
  env,
  future::{ready, Ready},
  rc::Rc,
};

pub struct BasicAuth;
impl<S, B> Transform<S, ServiceRequest> for BasicAuth
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<EitherBody<B>>;
  type Error = Error;
  type InitError = ();
  type Transform = BasicAuthMiddleware<S>;
  type Future = Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    ready(Ok(BasicAuthMiddleware {
      service: Rc::new(service),
    }))
  }
}

pub struct BasicAuthMiddleware<S> {
  service: Rc<S>,
}
impl<S, B> Service<ServiceRequest> for BasicAuthMiddleware<S>
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<EitherBody<B>>;
  type Error = Error;
  type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

  forward_ready!(service);

  fn call(&self, req: ServiceRequest) -> Self::Future {
    let auth_success = basic_auth(req.headers().get("Authorization"));
    if !auth_success {
      let http_res = HttpResponse::Unauthorized().finish();
      let (http_req, _) = req.into_parts();
      let res = ServiceResponse::new(http_req, http_res);

      return (async move { Ok(res.map_into_right_body()) }).boxed_local();
    }

    let service = Rc::clone(&self.service);
    Box::pin(async move {
      let res = service.call(req).await?;
      Ok(res.map_into_left_body())
    })
  }
}

fn basic_auth(auth_header: Option<&HeaderValue>) -> bool {
  let auth_header = match auth_header {
    Some(header) => header.to_str(),
    None => return false,
  };
  let auth_header = match auth_header {
    Ok(header) => header,
    Err(_) => return false,
  };

  let auth = auth_header.split_whitespace().collect::<Vec<&str>>();
  if auth.len() != 2 {
    return false;
  }

  let auth = auth[1];
  let auth = decode_base64(auth.to_string()).unwrap();
  let auth = auth.split(':').collect::<Vec<&str>>();
  if auth.len() != 2 {
    return false;
  }

  let user = match env::var("USERNAME") {
    Ok(username) => username,
    Err(_) => return false,
  };
  let pass = match env::var("PASSWORD") {
    Ok(password) => password,
    Err(_) => return false,
  };
  let username = auth[0];
  let password = auth[1];

  if username == user && password == pass {
    return true;
  }
  false
}
