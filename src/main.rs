mod digestor;
mod env;
mod middleware;
mod scheduler;
mod schemas;
mod storage;
mod utils;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use schemas::SchemaVariant;
use serde::Deserialize;
use std::io::{Read, Write};
use storage::{blob::Blob, manifest::Manifest};
use utils::ansi::{RESET, UNDERLINE};
use utils::verify_reference;
use uuid::Uuid;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  dotenv().ok();
  env::print_env_info();

  let scheduler = scheduler::start_scheduler().await;
  if let Err(e) = scheduler {
    println!("Scheduler error: {e}");
  }

  let port = env::port();
  let host = env::host();
  println!("Listening on {UNDERLINE}{}://{host}:{port}/{RESET}", env::PROTOCOL);

  HttpServer::new(|| {
    App::new()
      .wrap(middleware::log_requests::LogRequests)
      .wrap(middleware::basic_auth::BasicAuth)
      .app_data(web::PayloadConfig::new(1024 * 1024 * 1024)) // 1GB
      .route("/", web::get().to(|| async { env::crate_info() }))
      .route("/v2/", web::get().to(|| async { "Authenticated" }))
      .route("/v2/{name:[^{}]+}/blobs/{digest}", web::get().to(get_blob))
      .route("/v2/{name:[^{}]+}/blobs/{digest}", web::head().to(head_blob))
      .route("/v2/{name:[^{}]+}/manifests/{reference}", web::get().to(get_manifest))
      .route("/v2/{name:[^{}]+}/manifests/{reference}", web::head().to(head_manifest))
      .route("/v2/{name:[^{}]+}/blobs/uploads/", web::post().to(post_blob_upload))
      .route("/v2/{name:[^{}]+}/blobs/uploads/{reference}", web::put().to(put_blob_upload))
      .route("/v2/{name:[^{}]+}/blobs/uploads/{reference}", web::patch().to(patch_blob_upload))
      .route("/v2/{name:[^{}]+}/manifests/{reference}", web::put().to(put_manifest))
      .route("/v2/{name:[^{}]+}/tags/list", web::get().to(get_tags_list))
      .route("/v2/{name:[^{}]+}/manifests/{reference}", web::delete().to(delete_manifest))
      .route("/v2/{name:[^{}]+}/blobs/{digest}", web::delete().to(delete_blob))
      .route("/v2/{name:[^{}]+}/blobs/uploads/{reference}", web::get().to(get_blob_upload))
  })
  .bind((host, port))?
  .run()
  .await
}

/// end-2: `GET /v2/<name>/blobs/<digest>` => 200 / 404
///
/// RESPONSE:
/// - Docker-Content-Digest: {blob digest}
/// - Body: {blob data}
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-blobs>
async fn get_blob(path: web::Path<(String, String)>) -> impl Responder {
  let (name, digest) = path.into_inner();

  let blob = match Blob::new(name.clone(), digest.clone()) {
    Ok(blob) => blob,
    Err(e) => {
      println!("Error: Invalid blob request: {e:?}");
      return HttpResponse::BadRequest().finish();
    }
  };

  let mut file = match blob.read() {
    Ok(file) => file,
    Err(e) => {
      println!("Error getting blob: {e:?}");
      return HttpResponse::NotFound().finish();
    }
  };

  let mut buf = Vec::new();
  file.read_to_end(&mut buf).unwrap();

  // A successful response SHOULD contain the digest of the uploaded blob in the
  // header Docker-Content-Digest. If present, the value of this header MUST be
  // a digest matching that of the response body.

  // A GET request to an existing blob URL MUST provide the expected blob, with
  // a response code that MUST be 200 OK.
  HttpResponse::Ok().insert_header(("Docker-Content-Digest", digest)).body(buf)
}

/// end-2: `HEAD /v2/<name>/blobs/<digest>` => 200 / 404
///
/// RESPONSE:
/// - Docker-Content-Digest: {blob digest}
/// - Content-Length: {blob size in bytes}
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#checking-if-content-exists-in-the-registry>
async fn head_blob(path: web::Path<(String, String)>) -> impl Responder {
  let (name, digest) = path.into_inner();

  let blob = match Blob::new(name.clone(), digest.clone()) {
    Ok(blob) => blob,
    Err(e) => {
      println!("Error: Invalid blob request: {e:?}");
      return HttpResponse::BadRequest().finish();
    }
  };
  let file = match blob.read() {
    Ok(file) => file,
    Err(e) => {
      println!("Error getting blob: {e:?}");
      // If the blob or manifest is not found in the registry, the response code
      // MUST be `404 Not Found`.
      return HttpResponse::NotFound().finish();
    }
  };
  let metadata = file.metadata().unwrap();
  let content_length = metadata.len();

  // - A HEAD request to an existing blob or manifest URL MUST return `200 OK`.
  // - A successful response SHOULD contain the digest of the uploaded blob in
  //   the header `Docker-Content-Digest`.
  // - A successful response SHOULD contain the size in bytes of the uploaded
  //   blob in the header `Content-Length`.
  HttpResponse::Ok()
    .insert_header(("Docker-Content-Digest", digest))
    .insert_header(("Content-Length", content_length.to_string()))
    .finish()
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

  if verify_reference(reference.to_string()).is_err() {
    // NOTE: The spec doesn't mention what to do if the reference is invalid.
    println!("Error: Invalid reference: {reference:?}");
    return HttpResponse::BadRequest().finish();
  }

  let mut file = match storage::get_manifest(&name, &reference) {
    Ok(file) => file,
    Err(e) => match e.kind() {
      std::io::ErrorKind::NotFound => {
        println!("Error: Manifest not found: name='{name}' reference='{reference}'");
        return HttpResponse::NotFound().finish();
      }
      _ => {
        println!("Error getting manifest: {e:?}");
        return HttpResponse::InternalServerError().finish();
      }
    },
  };
  let mut data = Vec::new();
  file.read_to_end(&mut data).unwrap();

  let digest = digestor::get_sha256_digest(&data);

  // In a successful response, the Content-Type header will indicate the type of
  // the returned manifest.
  let content_type =
    storage::xattr::get_xattr_media_type(&file).unwrap_or(String::from("application/json"));

  HttpResponse::Ok()
    .insert_header(("Docker-Content-Digest", digest))
    .content_type(content_type)
    .body(data)
}

/// end-3: `HEAD /v2/<name>/manifests/<reference>` => 200 / 404
///
/// RESPONSE:
/// - Docker-Content-Digest: {manifest digest}
/// - Content-Length: {manifest size in bytes}
/// - Content-Type: {content type}           (see spec / content-negotiation.md)
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#checking-if-content-exists-in-the-registry>
async fn head_manifest(path: web::Path<(String, String)>) -> impl Responder {
  let (name, reference) = path.into_inner();

  if verify_reference(reference.to_string()).is_err() {
    // NOTE: The spec doesn't mention what to do if the reference is invalid.
    println!("Error: Invalid reference: {reference:?}");
    return HttpResponse::BadRequest().finish();
  }

  let mut file = match storage::get_manifest(&name, &reference) {
    Ok(file) => file,
    Err(e) => match e.kind() {
      std::io::ErrorKind::NotFound => {
        println!("Error: Manifest not found: name='{name}' reference='{reference}'");
        // If the blob or manifest is not found in the registry, the response
        // code MUST be `404 Not Found`.
        return HttpResponse::NotFound().finish();
      }
      _ => {
        println!("Error getting manifest: {e:?}");
        return HttpResponse::InternalServerError().finish();
      }
    },
  };

  // NOTE: The spec says _blobs_ should have the `Docker-Content-Digest` and
  // `Content-Length` headers, but does not mention manifests. It is however
  // expected by Containerization, so they are included here in addition to the
  // `Content-Type` header for compatibility.
  // <https://github.com/apple/containerization/blob/28b97f2917a9e25dce4591ad9d44c72968c5392f/Sources/ContainerizationOCI/Client/RegistryClient%2BFetch.swift#L58>

  // A successful response SHOULD contain the size in bytes of the uploaded blob
  // in the header `Content-Length`.
  let content_length = file.metadata().unwrap().len();

  let content_type =
    storage::xattr::get_xattr_media_type(&file).unwrap_or(String::from("application/json"));

  // A successful response SHOULD contain the digest of the uploaded blob in the
  // header `Docker-Content-Digest`.
  let mut data = Vec::new();
  file.read_to_end(&mut data).unwrap();
  let digest = digestor::get_sha256_digest(&data);

  // A HEAD request to an existing blob or manifest URL MUST return `200 OK`.
  HttpResponse::Ok()
    .insert_header(("Docker-Content-Digest", digest))
    .insert_header(("Content-Length", content_length.to_string()))
    .content_type(content_type)
    .finish()
}

#[derive(Deserialize)]
struct PostBlobParameters {
  digest: Option<String>,
  mount: Option<String>,
  #[allow(dead_code)]
  from: Option<String>,
}
/// end-4: `POST /v2/<name>/blobs/uploads/?digest=<digest>` => 201/202 / 404/400
/// end-11: `POST /v2/<name>/blobs/uploads/?mount=<digest>&from=<other_name>` => 201 / 404
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
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put>
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#mounting-a-blob-from-another-repository>
async fn post_blob_upload(
  path: web::Path<String>,
  query: web::Query<PostBlobParameters>,
  data: web::Bytes,
) -> impl Responder {
  let name = path.into_inner();
  if let Some(mount) = &query.mount {
    // end-11
    // <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#mounting-a-blob-from-another-repository>

    // If a necessary blob exists already in another repository within the same
    // registry, it can be mounted into a different repository.
    //
    // - <name> is the namespace to which the blob will be mounted.
    // - <mount> is the digest of the blob to mount.
    // - <from> is the namespace from which the blob should be mounted.
    //
    // The registry MAY treat the from parameter as optional, and it MAY cross-
    // mount the blob if it can be found.

    let blob = match Blob::new(name.clone(), mount.clone()) {
      Ok(blob) => blob,
      Err(e) => {
        println!("Error: Invalid blob: {e:?}");
        return HttpResponse::BadRequest().finish();
      }
    };
    match blob.mount() {
      Ok(_) => (),
      Err(e) => {
        // TODO: Conformance test "Cross-mounting of a blob without the from
        // argument should yield session id" fails because we don't response
        // with 202 here when the `from` parameter is not provided. However, the
        // spec doesn't specify what to do when treating the `from` parameter as
        // optional.

        println!("Error: Could not mount blob: {e:?}");
        return HttpResponse::NotFound().finish();
      }
    }

    // The response to a successful mount MUST be 201 Created, and MUST contain
    // the following header: `Location: <blob-location>`. The Location header
    // will contain the registry URL to access the accepted layer file.
    let location = format!("/v2/{name}/blobs/{mount}");

    return HttpResponse::Created().append_header(("Location", location)).finish();
  }

  match &query.digest {
    None => {
      // end-4a
      // <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put>
      // <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pushing-a-blob-in-chunks>

      // If the digest parameter is not provided, we are in the "POST then PUT"
      // or chunked upload flow (PATCH). We return the `location` header, which
      // points to an endpoint that accepts PUT <location>?digest=<digest>
      // (end-6) and PATCH <location> (end-5).

      // MUST contain a UUID representing a unique session ID
      let reference = Uuid::new_v4().to_string();

      let _ = storage::create_hunk(&name, &reference);

      let location = format!("/v2/{name}/blobs/uploads/{reference}");

      // Upon success, the response MUST have a code of 202 Accepted
      HttpResponse::Accepted().insert_header(("Location", location)).finish()
    }
    Some(digest) => {
      // end-4b
      // <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#single-post>
      let blob = match Blob::new(name.clone(), digest.clone()) {
        Ok(blob) => blob,
        Err(e) => {
          println!("Error: Invalid blob: {e:?}");
          return HttpResponse::BadRequest().finish();
        }
      };
      if let Err(e) = blob.verify(&data) {
        println!("Error: {e:?}");
        return HttpResponse::BadRequest().finish();
      }
      match blob.create() {
        Ok(mut file) => file.write_all(&data).unwrap(),
        Err(e) => {
          if e.kind() == std::io::ErrorKind::AlreadyExists {
            // We have already stored this blob. Until the spec tells us what to
            // do in this case, we treat it as a success and continue the normal
            // flow.
            println!("Warning: Existing blob uploaded, name='{name}' digest='{digest}'");
          } else {
            println!("Error: Could not create blob: {e:?}");
            return HttpResponse::InternalServerError().finish();
          }
        }
      };

      let location = format!("/v2/{name}/blobs/{digest}");

      // Successful completion of the request MUST return a 201 Created status
      // code.
      HttpResponse::Created().insert_header(("Location", location)).finish()
    }
  }
}

/// end-5: `PATCH /v2/<name>/blobs/uploads/<reference>` => 202 / 404/416
///
/// NOTE: This should be referred to from a preceeding POST request to end-4a:
///
///  > For information on obtaining a session ID, reference the above section on
///  > pushing a blob monolithically via the POST/PUT method. The process
///  > remains unchanged for chunked upload, except that the POST request MUST
///  > include the following header: `Content-Length: 0`
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
  req: HttpRequest,
  data: web::Bytes,
) -> impl Responder {
  let (name, reference) = path.into_inner();
  let content_range = req.headers().get("Content-Range");
  let (range, range_start, range_end) = match utils::get_content_range(content_range) {
    Some(range) => range,
    None => {
      if content_range.is_some() {
        println!("Error: Invalid range: {content_range:?}");
        return HttpResponse::BadRequest().finish();
      }

      // NOTE: The spec isn't clear about the case where the Content-Range
      // header is missing. But since the conformance tests and tools like
      // podman excludes it when the entire blob is being uploaded
      // monolithically, we'll assume that no range means the entire blob.
      // <https://github.com/opencontainers/distribution-spec/issues/506>
      let range_start = 0;
      let range_end = data.len() - 1;
      (format!("{range_start}-{range_end}"), range_start, range_end)
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

  let hunk = match storage::read_hunk(&name, &reference) {
    Ok(hunk) => hunk,
    Err(e) => {
      println!("Error: Could not get hunk: {e:?}");
      return HttpResponse::NotFound().finish();
    }
  };

  let size_in_bytes = hunk.metadata().unwrap().len();
  drop(hunk);

  if size_in_bytes == 0 && range_start != 0 {
    // The first chunk's range MUST begin with 0.
    println!("Error: First chunk's range must begin with 0: {range:?}");
    return HttpResponse::RangeNotSatisfiable().finish();
  } else if size_in_bytes != 0 && size_in_bytes != (range_start as u64) {
    // Chunks MUST be uploaded in order, with the first byte of a chunk being
    // the last chunk's <end-of-range> plus one. If a chunk is uploaded out of
    // order, the registry MUST respond with a 416 Requested Range Not
    // Satisfiable code.
    println!("Error: Uploaded hunk range did not match stored hunk: Stored: {size_in_bytes}, req_first_byte: {range_start}");
    return HttpResponse::RangeNotSatisfiable().finish();
  }

  if range_start + req_length != range_end + 1 {
    // The Content-Range header MUST specify the range of bytes being uploaded
    // in the format `0-{end-of-range}`.
    println!("Error: Invalid range: {range} ({range_start}+{req_length}!={range_end}+1)");
    return HttpResponse::BadRequest().finish();
  }

  let mut hunk = storage::append_hunk(&name, &reference).unwrap();
  hunk.write_all(&data).unwrap();

  // Each successful chunk upload MUST have a 202 Accepted response code, and
  // MUST have the following headers:
  // - Location: <location>
  // - Range: 0-<end-of-range>
  let location = format!("/v2/{name}/blobs/uploads/{reference}");
  let range = format!("0-{range_end}");

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
    let mut hunk = storage::append_hunk(&name, &reference).unwrap();
    hunk.write_all(&data).unwrap();
  }

  let digest = query.digest.as_str();
  match storage::commit_hunk(&name, &reference, digest) {
    Ok(_) => (),
    Err(e) => {
      println!("Error: Could not commit hunk: {e:?}");
      return HttpResponse::InternalServerError().finish();
    }
  };

  let blob_location = format!("/v2/{name}/blobs/{digest}");
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
async fn put_manifest(
  path: web::Path<(String, String)>,
  req: HttpRequest,
  data: web::Bytes,
) -> impl Responder {
  let (name, reference) = path.into_inner();

  let manifest = match Manifest::new(&name, &reference) {
    Ok(manifest) => manifest,
    Err(e) => {
      // NOTE: The spec doesn't mention what to do if the reference is invalid.
      println!("Error: Invalid manifest: {e:?}");
      return HttpResponse::BadRequest().finish();
    }
  };

  // Clients SHOULD set the Content-Type header to the type of the manifest
  // being pushed.
  let content_type = utils::get_content_type(req.headers().get("Content-Type"));
  let manifest_variant = match schemas::validate_manifest_data(data.to_vec(), content_type.clone())
  {
    Ok(manifest_variant) => manifest_variant,
    Err(e) => {
      println!("Error: Invalid manifest: {e:?}");
      return HttpResponse::BadRequest().finish();
    }
  };

  match manifest_variant {
    SchemaVariant::ImageManifest(manifest) => SchemaVariant::ImageManifest(manifest),
    SchemaVariant::ImageIndex(index) => SchemaVariant::ImageIndex(index),
    SchemaVariant::Unknown(base) => {
      println!("Error: Unknown manifest: {base:?}");
      return HttpResponse::BadRequest().finish();
    }
  };

  match manifest.create_manifest(data.to_vec(), content_type) {
    // The registry MUST store the manifest in the exact byte representation
    // provided by the client.
    Ok(_) => (),
    Err(e) => {
      println!("Error: Could not create manifest: {e:?}");
      if e.kind() == std::io::ErrorKind::InvalidData {
        return HttpResponse::BadRequest().finish();
      }
      return HttpResponse::InternalServerError().finish();
    }
  };

  let location = format!("/v2/{name}/manifests/{reference}");
  HttpResponse::Created().insert_header(("Location", location)).finish()
}

#[derive(Deserialize)]
struct GetTagsListParameters {
  n: Option<usize>,
  last: Option<String>,
}
/// end-8: `GET /v2/<name>/tags/list` => 200 / 404
///
/// RESPONSE: (`skopeo list-tags docker://docker.io/rosvik/container-cubby`)
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

  let mut tags = match storage::get_tags(&name) {
    Ok(tags) => tags,
    Err(e) => {
      if e.kind() == std::io::ErrorKind::NotFound {
        return HttpResponse::NotFound().finish();
      } else {
        println!("Error: Could not get tags: {e:?}");
        return HttpResponse::InternalServerError().finish();
      }
    }
  };

  // The list of tags MAY be empty if there are no tags on the repository.
  // If the list is not empty, the tags MUST be in lexical order (i.e.
  // case-insensitive alphanumeric order).
  //
  // NOTE: The order in which fs::read_dir returns entries is platform and
  // filesystem dependent.
  tags.sort_by_key(|tag| tag.to_lowercase());

  // A subset of the tags can be fetched by providing the n query parameter.
  // The last query parameter provides further means for limiting the number of
  // tags. <tagname> will not be included in the results, but up to <int> tags
  // after <tagname> will be returned.
  let tags = utils::get_tag_range(tags, count, query.last.as_deref());
  let tag_count = tags.len();

  let link = format!(
    "</v2/{name}/tags/list?n={count}&last={last}>; rel=\"next\"",
    name = name,
    count = tag_count,
    last = tags.last().unwrap()
  );

  // <name> is the namespace of the repository. Assuming a repository is found,
  // this request MUST return a 200 OK response code.
  let tags_list = schemas::TagsList { name, tags };

  if tag_count > count {
    // The response MAY return fewer than n results, but only when the total
    // number of tags attached to the repository is less than n or a Link header
    // is provided.
    HttpResponse::Ok()
      .insert_header(("Link", link))
      .content_type("application/json")
      .body(serde_json::to_string(&tags_list).unwrap())
  } else {
    HttpResponse::Ok()
      .content_type("application/json")
      .body(serde_json::to_string(&tags_list).unwrap())
  }
}

/// end-9: `DELETE /v2/<name>/manifests/<reference>` => 202 / 404
///
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#deleting-tags>
/// <https://github.com/opencontainers/distribution-spec/blob/main/spec.md#deleting-manifests>
async fn delete_manifest(path: web::Path<(String, String)>) -> impl Responder {
  let (name, reference) = path.into_inner();
  match storage::delete_manifest(&name, &reference) {
    Ok(_) => (),
    Err(e) => {
      println!(
        "Warning: Error '{e}' when deleting manifest, name='{name}' reference='{reference}'"
      );
      return HttpResponse::NotFound().finish();
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

  let blob = match Blob::new(name.clone(), digest.clone()) {
    Ok(blob) => blob,
    Err(e) => {
      println!("Error: Invalid blob reqest: {e:?}");
      return HttpResponse::BadRequest().finish();
    }
  };

  // NOTE: This does not delete the blob file itself, only the symlink, in case
  // it is in use by another container. Blob file deletion is handled by the
  // prune job instead.
  match blob.unmount() {
    Ok(_) => {
      // Upon success, the registry MUST respond with code 202 Accepted.
      HttpResponse::Accepted().finish()
    }
    Err(e) => {
      println!("Warning: Error '{e}' when deleting blob, name='{name}' digest='{digest}'");
      HttpResponse::NotFound().finish()
    }
  }
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

  let hunk = match storage::read_hunk(&name, &reference) {
    Ok(hunk) => hunk,
    Err(e) => {
      println!("Error: Could not get hunk: {e:?}");
      return HttpResponse::NotFound().finish();
    }
  };

  // The <location> refers to the URL obtained from any preceding POST or PATCH
  // request.
  let location = format!("/v2/{name}/blobs/uploads/{reference}");

  // The <end-of-range> value is the position of the last uploaded byte.
  let size_in_bytes = hunk.metadata().unwrap().len();
  let range = format!("0-{}", size_in_bytes - 1);

  // The response to an active upload <location> MUST be a 204 No Content
  // response code
  HttpResponse::NoContent()
    .insert_header(("Location", location))
    .insert_header(("Range", range))
    .finish()
}

#[cfg(test)]
mod tests;
