use crate::utils::decode_base64;
use actix_web::{
  body::EitherBody,
  dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
  http::header::HeaderValue,
  http::Method,
  Error, HttpResponse,
};
use futures_util::{future::LocalBoxFuture, FutureExt};
use std::{
  env,
  future::{ready, Ready},
  rc::Rc,
};

#[derive(Debug, Clone, PartialEq)]
pub enum AuthMode {
  None,
  ReadWrite,
  WriteOnly,
}

#[derive(Clone)]
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
    let auth_mode = get_auth_mode();

    #[allow(clippy::if_same_then_else)]
    let requires_auth = if auth_mode == AuthMode::None {
      false // Auth is completely disabled
    } else if auth_mode == AuthMode::ReadWrite {
      true // Require auth for all requests
    } else if auth_mode == AuthMode::WriteOnly && req.path().eq("/v2/") {
      true // Issue auth challenge for root endpoint
    } else if auth_mode == AuthMode::WriteOnly {
      !matches!(*req.method(), Method::GET | Method::HEAD) // Allow read operations
    } else {
      panic!("Invalid state in basic_auth: MODE={:?}, METHOD={}", auth_mode, *req.method());
    };

    if requires_auth {
      let server_credentials = match (env::var("USERNAME"), env::var("PASSWORD")) {
        (Ok(username), Ok(password)) => format!("{username}:{password}"),
        _ => {
          // If server credentials are not set, return 401 Unauthorized.
          let response = into_unauthorized(req);
          return (async move { Ok(response.map_into_right_body()) }).boxed_local();
        }
      };

      let auth_success = basic_auth(req.headers().get("Authorization"), server_credentials);
      if !auth_success {
        let response = into_auth_challenge(req);
        return (async move { Ok(response.map_into_right_body()) }).boxed_local();
      }
    }

    let service = Rc::clone(&self.service);
    Box::pin(async move {
      let res = service.call(req).await?;
      Ok(res.map_into_left_body())
    })
  }
}

fn basic_auth(auth_header: Option<&HeaderValue>, server_credentials: String) -> bool {
  let auth_header = match auth_header {
    Some(header) => header.to_str().unwrap_or_default(),
    _ => return false,
  };

  let auth = auth_header.split_whitespace().collect::<Vec<&str>>();
  if auth.len() != 2 {
    return false;
  }
  if auth[0] != "Basic" {
    return false;
  }
  let auth = auth[1];
  let request_credentials = decode_base64(auth.to_string()).unwrap_or_default();

  request_credentials == server_credentials
}

fn into_auth_challenge(req: ServiceRequest) -> ServiceResponse {
  let http_res = HttpResponse::Unauthorized()
    .insert_header(("WWW-Authenticate", "Basic realm=\"\", charset=\"UTF-8\""))
    .finish();
  let (http_req, _) = req.into_parts();
  ServiceResponse::new(http_req, http_res)
}

fn into_unauthorized(req: ServiceRequest) -> ServiceResponse {
  let http_res = HttpResponse::Unauthorized().finish();
  let (http_req, _) = req.into_parts();
  ServiceResponse::new(http_req, http_res)
}

fn get_auth_mode() -> AuthMode {
  match env::var("AUTH_MODE").unwrap().as_str() {
    "none" => AuthMode::None,
    "read_write" => AuthMode::ReadWrite,
    "write_only" => AuthMode::WriteOnly,
    invalid => panic!("Invalid AUTH_MODE: {invalid}"),
  }
}
