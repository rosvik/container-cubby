mod db;
mod digestor;
mod manifest;
mod middleware;
mod utils;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use db::CommitHunkError;
use dotenv::dotenv;
use manifest::Manifest;
use serde::Deserialize;
use std::env;
use utils::{verify_blob, verify_reference};
use uuid::Uuid;

const PROTOCOL: &str = "http";
const HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 8602;
const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  dotenv().ok();
  let port = env::var("PORT").unwrap_or_default().parse::<u16>().unwrap_or(DEFAULT_PORT);
  let addr = format!("{}:{}", HOST, port);

  dotenv().ok();
  if env::var("USERNAME").is_err() || env::var("PASSWORD").is_err() {
    println!(
      "\x1b[1;33mINFO: Username/password was not provided. Registry is in read-only mode.\x1b[0m"
    );
  };

  db::init().unwrap();

  println!("Listening on \x1b[1;4m{PROTOCOL}://{addr}/\x1b[0m");
  HttpServer::new(|| {
    // let basic_auth = axum::middleware::from_fn(middleware::basic_authenticate);
    // let upload_middleware =
    //   ServiceBuilder::new().layer(basic_auth.clone()).layer(unlimited_upload_size.clone());
    App::new()
      .wrap(middleware::log_requests::LogRequests)
      .route("/", web::get().to(|| async { format!("{CRATE_NAME} v{CRATE_VERSION}") }))
      .route("/v2/", web::get().to(|| async { "Authenticated" })) //.layer(basic_auth.clone()))
      .route("/v2/{name}/blobs/{digest}", web::get().to(get_blob))
      .route("/v2/{name}/manifests/{reference}", web::get().to(get_manifest))
      .route(
        "/v2/{name}/blobs/uploads/",
        web::post().to(post_blob_upload), //.layer(upload_middleware.clone()),
      )
      .route(
        "/v2/{name}/blobs/uploads/{reference}",
        web::put().to(put_blob_upload), //.layer(upload_middleware.clone()),
      )
      .route(
        "/v2/{name}/blobs/uploads/{reference}",
        web::patch().to(patch_blob_upload), //.layer(upload_middleware.clone()),
      )
      .route("/v2/{name}/manifests/{reference}", web::put().to(put_manifest)) //.layer(upload_middleware.clone()))
      .route("/v2/{name}/tags/list", web::get().to(get_tags_list))
      .route("/v2/{name}/manifests/{reference}", web::delete().to(delete_manifest)) //.layer(basic_auth.clone()))
      .route("/v2/{name}/blobs/{digest}", web::delete().to(delete_blob)) //.layer(basic_auth.clone()))
      .route("/v2/{name}/blobs/uploads/{reference}", web::get().to(get_blob_upload))
  })
  .bind((HOST, port))?
  .run()
  .await
}

/// end-2: `GET /v2/<name>/blobs/<digest>` => 200 / 404
///
/// RESPONSE:
/// - Docker-Content-Digest: {the blob's digest}
/// - Body: {blob data}
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-blobs>
async fn get_blob(path: web::Path<(String, String)>) -> impl Responder {
  let (name, digest) = path.into_inner();
  let conn = db::connect().unwrap();
  let blob = match db::get_blob(&conn, &name, &digest) {
    Ok(blob) => blob,
    Err(e) => {
      // If the blob is not found in the registry, the response code MUST be
      // 404 Not Found.
      println!("Error getting blob: {:?}", e);
      // return (StatusCode::NOT_FOUND, HeaderMap::new(), "".into());
      return HttpResponse::NotFound().finish();
    }
  };

  // A successful response SHOULD contain the digest of the uploaded blob in the
  // header Docker-Content-Digest. If present, the value of this header MUST be
  // a digest matching that of the response body.

  // A GET request to an existing blob URL MUST provide the expected blob, with
  // a response code that MUST be 200 OK.
  HttpResponse::Ok().insert_header(("Docker-Content-Digest", blob.digest)).body(blob.data.unwrap())
}

/// end-3: `GET /v2/<name>/manifests/<reference>` => 200 / 404
///
/// REQUEST:
/// - Accept: {content type}                 (see spec / content-negotiation.md)
///
/// RESPONSE:
/// - Content-Type: {content type}           (see spec / content-negotiation.md)
/// - Docker-Content-Digest: {digest}    (canonical digest of the uploaded blob)
/// - Body: {manifest}
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests>
async fn get_manifest(path: web::Path<(String, String)>) -> impl Responder {
  let (name, reference) = path.into_inner();
  let conn = db::connect().unwrap();
  let manifest = match db::get_manifest(&conn, &name, &reference) {
    Ok(manifest) => manifest,
    Err(e) => {
      println!("Error getting manifest: {:?}", e);
      // return (StatusCode::NOT_FOUND, HeaderMap::new(), "".into());
      return HttpResponse::NotFound().finish();
    }
  };

  let digest = digestor::get_sha256_digest(&manifest.data.clone().unwrap());

  // In a successful response, the Content-Type header will indicate the type of
  // the returned manifest.
  let content_type = "application/vnd.oci.image.manifest.v1+json";

  HttpResponse::Ok()
    .insert_header(("Docker-Content-Digest", digest))
    .content_type(content_type)
    .body(manifest.data.unwrap())
}

#[derive(Deserialize)]
struct PostBlobParameters {
  digest: Option<String>,
}
/// end-4: `POST /v2/<name>/blobs/uploads/?digest=<digest>` => 201/202 / 404/400
///
/// REQUEST
/// - Content-Length: {length}          (must match the blob's actual content length)
/// - Content-Type: `application/octet-stream`
/// - Body: {blob byte stream}
///
/// RESPONSE
/// - Location: {blob-location}         (a pullable blob URL)
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#single-post>

async fn post_blob_upload(
  path: web::Path<String>,
  query: web::Query<PostBlobParameters>,
  data: web::Bytes,
) -> impl Responder {
  let name = path.into_inner();
  let conn = db::connect().unwrap();
  let digest = match &query.digest {
    Some(digest) => {
      // end-4b
      if verify_blob(&data, digest.as_str()).is_err() {
        return HttpResponse::BadRequest().finish();
      }
      digest
    }
    None => {
      // end-4a

      // If the digest parameter is not provided, we are in the "POST then PUT"
      // or chunked upload flow (PATCH). We return the `location` header, which
      // points to an endpoint that accepts PUT <location>?digest=<digest>
      // (end-6) and PATCH <location> (end-5).
      // <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put>
      // <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-a-blob-in-chunks>

      // MUST contain a UUID representing a unique session ID
      let reference = Uuid::new_v4().to_string();
      db::insert_empty_hunk(&conn, &name, &reference).unwrap();

      // let mut headers = HeaderMap::new();
      // headers.insert("Location", HeaderValue::from_str(location.as_str()).unwrap());
      let location = format!("/v2/{}/blobs/uploads/{}", name, reference);

      // Upon success, the response MUST have a code of 202 Accepted
      // return (StatusCode::ACCEPTED, headers, ());
      return HttpResponse::Accepted().insert_header(("Location", location)).finish();
    }
  };

  match db::insert_blob(&conn, name.as_str(), digest.as_str(), &data) {
    Ok(_) => (),
    Err(e) => {
      if e == rusqlite::Error::InvalidQuery {
        // If the request is invalid, such as a <digest> with an invalid syntax,
        // a 400 Bad Request MUST be returned.
        return HttpResponse::BadRequest().finish();
      }
      println!("Error inserting blob: {:?}", e);
      // return (StatusCode::INTERNAL_SERVER_ERROR, HeaderMap::new(), ());
      return HttpResponse::InternalServerError().finish();
    }
  }

  let location = format!("/v2/{name}/blobs/{digest}");

  // Successful completion of the request MUST return a 201 Created status code.
  HttpResponse::Created().insert_header(("Location", location)).finish()
}

/// end-5: `PATCH /v2/<name>/blobs/uploads/<reference>` => 202 / 404/416
///
/// NOTE: This should be referred to from a preceeding POST request to end-4a:
///
///  > For information on obtaining a session ID, reference the above
///    section on pushing a blob monolithically via the POST/PUT method.
///    The process remains unchanged for chunked upload, except that the
///    post request MUST include the following header:
///
/// REQUEST
/// - Content-Type: `application/octet-stream`
/// - Content-Range: {range}   (byte range of the chunk, inclusive on both ends)
/// - Content-Length: {length}    (must match the chunk's actual content length)
/// - Body: {chunk byte stream}
///
/// RESPONSE
/// - Location: {location}                        (url to the next chunk upload)
/// - Range: 0-{end-of-range}          (0 to position of the last uploaded byte)
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-a-blob-in-chunks>
async fn patch_blob_upload(
  path: web::Path<(String, String)>,
  // content_range: Option<web::Header<http::header::ContentRange>>,
  req: HttpRequest,
  data: web::Bytes,
) -> impl Responder {
  let (name, reference) = path.into_inner();
  let content_range = req.headers().get("Content-Range");
  let (range, range_start, range_end) = match utils::get_content_range(content_range) {
    Some(range) => range,
    None => {
      if content_range.is_some() {
        println!("Error: Invalid range: {:?}", content_range);
        return HttpResponse::BadRequest().finish();
      }

      // NOTE: The spec isn't clear about the case where the Content-Range
      // header is missing. But since the conformance tests and tools like
      // podman excludes it when the entire blob is being uploaded
      // monolithically, we'll assume that no range means the entire blob.
      // <https://github.com/opencontainers/distribution-spec/issues/506>
      let range_start = 0;
      let range_end = data.len() - 1;
      (format!("{}-{}", range_start, range_end), range_start, range_end)
    }
  };
  let content_length = req.headers().get("Content-Length");

  let req_length = match utils::get_content_length(content_length) {
    Some(length) => length,
    None => {
      // NOTE: This is a conformance error, but since clients doesn't always
      //       include it, we continue the flow.
      println!(
        "Warning: Not able to parse request content length from headers. Data is {} bytes.",
        data.len()
      );
      data.len()
    }
  };

  // Content-Length header MUST match the actual number of bytes in the chunk.
  if req_length != data.len() {
    println!("Error: Invalid content length: Content-Length={}, data={}", req_length, data.len());
    return HttpResponse::BadRequest().finish();
  }

  let conn = db::connect().unwrap();
  {
    let stored_hunk = match db::get_hunk(&conn, &name, &reference) {
      Ok(hunk) => hunk,
      Err(e) => {
        println!("Error: Could not get hunk: {:?}", e);
        return HttpResponse::NotFound().finish();
      }
    };

    if stored_hunk.last_byte.is_none() && range_start != 0 {
      // The first chunk's range MUST begin with 0.
      println!("Error: First chunk's range must begin with 0: {:?}", range);
      return HttpResponse::RangeNotSatisfiable().finish();
    } else if stored_hunk.last_byte.is_some() && stored_hunk.last_byte.unwrap() + 1 != range_start {
      // Chunks MUST be uploaded in order, with the first byte of a chunk being
      // the last chunk's <end-of-range> plus one. If a chunk is uploaded out of
      // order, the registry MUST respond with a 416 Requested Range Not
      // Satisfiable code.
      println!(
        "Error: Uploaded hunk range did not match stored hunk: Stored: {}, req_first_byte: {}",
        stored_hunk.last_byte.unwrap(),
        range_start
      );
      return HttpResponse::RangeNotSatisfiable().finish();
    }

    if range_start + req_length != range_end + 1 {
      // The Content-Range header MUST specify the range of bytes being uploaded
      // in the format `0-{end-of-range}`.
      println!("Error: Invalid range: {} ({range_start}+{req_length}!={range_end}+1)", range);
      return HttpResponse::BadRequest().finish();
    }
  }

  db::append_hunk(&conn, &name, &reference, data.to_vec()).unwrap();

  // Each successful chunk upload MUST have a 202 Accepted response code, and
  // MUST have the following headers:
  // - Location: <location>
  // - Range: 0-<end-of-range>
  let location = format!("/v2/{}/blobs/uploads/{}", name, reference);
  let range = format!("0-{}", range_end);

  HttpResponse::Accepted()
    .insert_header(("Location", location))
    .insert_header(("Range", range))
    .finish()
}

#[derive(Deserialize)]
struct PutBlobParameters {
  digest: String,
}
/// end-6: `PUT /v2/<name>/blobs/uploads/<reference>?digest=<digest>` => 201 / 404/400
///
/// REQUEST
/// - Content-Length: {length}         (must match blob or chunk content length)
/// - Content-Type: `application/octet-stream`
/// - Content-Range: {chunk range}     (if the blob is being uploaded in chunks)
/// - Body: {blob byte stream}
///
/// RESPONSE
/// - Location: {blob-location}                            (a pullable blob URL)
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put>
async fn put_blob_upload(
  path: web::Path<(String, String)>,
  query: web::Query<PutBlobParameters>,
  data: web::Bytes,
  req: HttpRequest,
) -> impl Responder {
  let (name, reference) = path.into_inner();
  let conn = db::connect().unwrap();
  let content_length = req.headers().get("Content-Length");
  let req_length = utils::get_content_length(content_length).unwrap_or(0);

  // Content-Length header MUST match the actual number of bytes in the chunk.
  if req_length != data.len() {
    println!("Error: Invalid content length: Content-Length={}, data={}", req_length, data.len());
    return HttpResponse::BadRequest().finish();
  }

  if !data.is_empty() {
    // TODO: Verify Content-Range

    // We have recieved the final hunk of a blob or the entire blob in one go
    db::append_hunk(&conn, &name, &reference, data.to_vec()).unwrap();
  }

  match db::commit_hunk(&conn, name.as_str(), &reference, query.digest.as_str()) {
    Ok(_) => (),
    Err(e) => {
      match e {
        CommitHunkError::DatabaseError(e) => {
          if e == rusqlite::Error::InvalidQuery {
            // If the request is invalid, such as a <digest> with an invalid syntax,
            // a 400 Bad Request MUST be returned.
            return HttpResponse::BadRequest().finish();
          }
          println!("Error inserting blob: {:?}", e);
          return HttpResponse::InternalServerError().finish();
        }
        CommitHunkError::DigestMismatch(e) => {
          // Query digest MUST match the blob's digest.
          println!("Error: Digest mismatch: {:?}", e);
          return HttpResponse::BadRequest().finish();
        }
      }
    }
  }

  let blob_location = format!("/v2/{name}/blobs/{}", query.digest);

  HttpResponse::Created().insert_header(("Location", blob_location)).finish()
}

/// end-7: `PUT /v2/<name>/manifests/<reference>` => 201 / 404
///
/// REQUEST
/// - Content-Type: {content type}           (same as mediaType in the manifest)
/// - Body: {manifest byte stream}
///
/// RESPONSE
/// - Location: {manifest-location}                    (a pullable manifest URL)
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-manifests>
async fn put_manifest(path: web::Path<(String, String)>, data: web::Bytes) -> impl Responder {
  let (name, reference) = path.into_inner();
  if verify_reference(&reference).is_err() {
    // NOTE: The spec doesn't mention what to do if the reference is invalid.
    return HttpResponse::BadRequest().finish();
  }

  match serde_json::from_slice::<Manifest>(&data) {
    Ok(manifest) => manifest,
    Err(e) => {
      println!("Error: Invalid manifest: {:?}", e);
      return HttpResponse::BadRequest().finish();
    }
  };

  let conn = db::connect().unwrap();
  match db::insert_manifest(&conn, &name, &reference, data.to_vec()) {
    Ok(_) => {}
    Err(e) => {
      if e.sqlite_error_code() != Some(rusqlite::ErrorCode::ConstraintViolation) {
        return HttpResponse::InternalServerError().finish();
      }
      // We have already stored this blob. Until the spec tells us what to do in
      // this case, we treat it as a success and continue the normal flow.
      println!("Warning: Duplicate manifest, name='{}' reference='{}'", name, reference);
    }
  };

  let location = format!("/v2/{name}/manifests/{reference}");
  HttpResponse::Created().insert_header(("Location", location)).finish()
}

#[derive(Deserialize)]
struct GetTagsListParameters {
  n: Option<usize>,
}
/// end-8: `GET /v2/<name>/tags/list` => 200 / 404
///
/// RESPONSE: (`skopeo list-tags docker://docker.io/rosvik/tiny-registry`)
/// - Content-Type: `application/json`
/// - Link: {RFC5988 with rel="next"}                   (if there are more tags)
/// - Body:
/// ```
/// {
///   "Repository": {name},
///   "Tags": [ {list of tags} ]
/// }
/// ```
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#listing-tags>
async fn get_tags_list(
  path: web::Path<String>,
  query: web::Query<GetTagsListParameters>,
) -> impl Responder {
  let name = path.into_inner();
  // In addition to fetching the whole list of tags, a subset of the tags can be
  // fetched by providing the n query parameter.
  let count = query.n.unwrap_or(100).clamp(0, 100);

  let conn = db::connect().unwrap();
  let mut tags = match db::get_tags(&conn, &name) {
    Ok(tags) => tags,
    Err(e) => {
      println!("Error getting tags: {:?}", e);
      return HttpResponse::NotFound().finish();
    }
  };

  // The list of tags MAY be empty if there are no tags on the repository.
  // If the list is not empty, the tags MUST be in lexical order (i.e.
  // case-insensitive alphanumeric order).
  tags.sort_by_key(|tag| tag.to_lowercase());

  // A subset of the tags can be fetched by providing the n query parameter
  let tags = tags[..count].to_vec();

  // <name> is the namespace of the repository. Assuming a repository is found,
  // this request MUST return a 200 OK response code. The list of tags MAY be
  // empty if there are no tags on the repository. If the list is not empty, the
  // tags MUST be in lexical order (i.e. case-insensitive alphanumeric order).
  let tags_list = serde_json::json!({
    "Repository": name,
    "Tags": tags
  });

  // The response MAY return fewer than n results, but only when the total
  // number of tags attached to the repository is less than n or a Link header
  // is provided.
  if tags.len() > count {
    let link = format!(
      "</v2/{name}/tags/list?n={count}&last={last}>; rel=\"next\"",
      name = name,
      count = tags.len(),
      last = tags.last().unwrap()
    );
    HttpResponse::Ok()
      .insert_header(("Link", link))
      .content_type("application/json")
      .body(serde_json::to_string(&tags_list).unwrap());
  }

  HttpResponse::Ok()
    .content_type("application/json")
    .body(serde_json::to_string(&tags_list).unwrap())
}

/// end-9: `DELETE /v2/<name>/manifests/<reference>` => 202 / 404
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#deleting-manifests>
async fn delete_manifest(path: web::Path<(String, String)>) -> impl Responder {
  let (name, reference) = path.into_inner();
  let conn = db::connect().unwrap();
  match db::delete_manifest(&conn, &name, &reference) {
    Ok(num_rows_changed) => {
      // If the repository does not exist, the response MUST return 404 Not Found.
      if num_rows_changed == 0 {
        println!("Warning: Manifest not found, name='{}' reference='{}'", name, reference);
        return HttpResponse::NotFound().finish();
      }
    }
    Err(e) => {
      println!("Error deleting manifest: {:?}", e);
      return HttpResponse::InternalServerError().finish();
    }
  }

  // Upon success, the registry MUST respond with a 202 Accepted code.
  HttpResponse::Accepted().finish()
}

/// end-10: `DELETE /v2/<name>/blobs/<digest>` => 202 / 404
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#deleting-blobs>
async fn delete_blob(path: web::Path<(String, String)>) -> impl Responder {
  let (name, digest) = path.into_inner();
  let conn = db::connect().unwrap();
  match db::delete_blob(&conn, &name, &digest) {
    Ok(num_rows_changed) => {
      // If the blob is not found, a 404 Not Found code MUST be returned.
      if num_rows_changed == 0 {
        println!("Warning: Blob not found, name='{}' digest='{}'", name, digest);
        return HttpResponse::NotFound().finish();
      }
    }
    Err(e) => {
      println!("Error deleting blob: {:?}", e);
      return HttpResponse::InternalServerError().finish();
    }
  }

  // Upon success, the registry MUST respond with code 202 Accepted.
  HttpResponse::Accepted().finish()
}

/// end-13: `GET /v2/<name>/blobs/uploads/<reference>` => 200 / 404
///
/// RESPONSE:
/// - Location: {blob-location}                            (a pullable blob URL)
/// - Range: 0-{end-of-range}          (0 to position of the last uploaded byte)
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-a-blob-in-chunks>
async fn get_blob_upload(path: web::Path<(String, String)>) -> impl Responder {
  let (name, reference) = path.into_inner();
  // To get the current status after a 416 error, issue a GET request to a URL
  // <location> (end-13). The following chunk upload SHOULD use the <location>
  // provided in the response.

  let conn = db::connect().unwrap();
  let hunk = match db::get_hunk(&conn, &name, &reference) {
    Ok(hunk) => hunk,
    Err(e) => {
      println!("Error getting hunk: {:?}", e);
      return HttpResponse::NotFound().finish();
    }
  };

  // The <location> refers to the URL obtained from any preceding POST or PATCH
  // request.
  let location = format!("/v2/{}/blobs/uploads/{}", name, reference);

  // The <end-of-range> value is the position of the last uploaded byte.
  // NOTE: If the hunk is empty, end_of_range is set to -1 (`range: 0--1`). It's
  //       unclear what's the expected behavior in this case.
  let end_of_range: i64 = (hunk.data.unwrap_or(Vec::new()).len() as i64) - 1;
  let range = format!("0-{}", end_of_range);

  // The response to an active upload <location> MUST be a 204 No Content
  // response code
  HttpResponse::NoContent()
    .insert_header(("Location", location))
    .insert_header(("Range", range))
    .finish()
}

#[cfg(test)]
mod tests;
