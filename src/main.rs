mod db;
use axum::{
    extract::{Path, Query},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
/*
https://specs.opencontainers.org/distribution-spec/#endpoints
ID      Method      API Endpoint                                                Success  Failure
end-1   GET         /v2/                                                        200      404/401
end-2   GET / HEAD  /v2/<name>/blobs/<digest>                                   200      404
end-3   GET / HEAD  /v2/<name>/manifests/<reference>                            200      404
end-4a  POST        /v2/<name>/blobs/uploads/                                   202      404
end-4b  POST        /v2/<name>/blobs/uploads/?digest=<digest>                   201/202  404/400
end-5   PATCH       /v2/<name>/blobs/uploads/<reference>                        202      404/416
end-6   PUT         /v2/<name>/blobs/uploads/<reference>?digest=<digest>        201      404/400
end-7   PUT         /v2/<name>/manifests/<reference>                            201      404
end-8a  GET         /v2/<name>/tags/list                                        200      404
end-8b  GET         /v2/<name>/tags/list?n=<integer>&last=<integer>             200      404
end-9   DELETE      /v2/<name>/manifests/<reference>                            202      404/400/405
end-10  DELETE      /v2/<name>/blobs/<digest>                                   202      404/405
end-11  POST        /v2/<name>/blobs/uploads/?mount=<digest>&from=<other_name>  201      404
*/

const HOST: &str = "0.0.0.0:8602";
const PROTOCOL: &str = "http";

#[tokio::main]
async fn main() {
    db::init().unwrap();
    let router = Router::new()
        .route("/v2", get(()))
        .route("/v2/:name/blobs/:digest", get(get_blob))
        .route("/v2/:name/manifests/:reference", get(get_manifest))
        .route("/v2/:name/blobs/uploads/", post(post_blob));

    let listener = tokio::net::TcpListener::bind(HOST).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

// ID     Method  API Endpoint               Success
// end-2  GET     /v2/<name>/blobs/<digest>  200
async fn get_blob(Path((name, digest)): Path<(String, String)>) {
    println!("name: {}, digest: {}", name, digest);
}

// ID     Method  API Endpoint                      Success Failure
// end-3  GET     /v2/<name>/manifests/<reference>  200     404
async fn get_manifest(Path((name, reference)): Path<(String, String)>) {
    println!("name: {}, reference: {}", name, reference);
}

/*
https://github.com/opencontainers/distribution-spec/blob/main/spec.md#single-post

ID     Method  API Endpoint                                     Success  Failure
end-4b POST    /v2/<name>/blobs/uploads/?digest=<digest>        201/202  404/400

REQUEST
    Content-Length: <length>
    Content-Type: application/octet-stream
    <upload byte stream>

RESPONSE
    Location: <blob-location>    <- a pullable blob URL.
*/
#[derive(Deserialize)]
struct PostBlobParameters {
    digest: String,
}
async fn post_blob(
    Path(name): Path<String>,
    Query(query): Query<PostBlobParameters>,
    data: axum::body::Bytes,
) -> impl IntoResponse {
    let conn = db::connect().unwrap();
    let digest = query.digest;
    let data: Vec<u8> = data.to_vec();

    let mut headers = HeaderMap::new();
    headers.insert(
        "Location",
        HeaderValue::from_str(format!("{PROTOCOL}://{HOST}/v2/{}/blobs/{}", name, digest).as_str())
            .unwrap(),
    );

    // TODO: Verify digest towards data
    println!("body: {:?}", data);

    let res = match db::insert_blob(&conn, &digest, &data) {
        Ok(res) => res,
        Err(e) => {
            if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                // The blob already exists
                println!("Duplicate digest: {:?}", e);
                return (StatusCode::OK, headers, ());
            }
            println!("Error inserting blob: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, headers, ());
        }
    };

    println!("inserted row {}", res);

    (StatusCode::OK, headers, ())
}
