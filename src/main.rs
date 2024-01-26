use axum::{extract::Path, routing::get, Router};
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

#[tokio::main]
async fn main() {
    let router = Router::new()
        .route("/v2", get(()))
        .route("/v2/:name/blobs/:digest", get(get_blob))
        .route("/v2/:name/manifests/:reference", get(get_manifest));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8602").await.unwrap();
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
