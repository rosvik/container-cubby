# tiny-registry

Initial goals:

1. Recieve a blob from a client (POST `/v2/<name>/blobs/<digest>`)
	- https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-blobs
2. Store the blob in a a Sqlite database
	- https://lib.rs/crates/rusqlite
3. Serve the blob to a client (GET `/v2/<name>/blobs/uploads/?digest=<digest>`)
  - https://github.com/opencontainers/distribution-spec/blob/main/spec.md#single-post


## Spec links

- [OCI Image Format Specification](https://github.com/opencontainers/image-spec)
- [OCI Runtime Specification](https://github.com/opencontainers/runtime-spec)
- [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec)
- Google go-containerregistry have some good docs from the client's perspective
	- [google/go-containerregistry](https://github.com/google/go-containerregistry/blob/main/pkg/v1/remote/README.md)

| ID      | Method   | API Endpoint                                                 | Success | Failure     | Todo |
| ------- | -------- | ------------------------------------------------------------ | ------- | ----------- | ---- |
| end-1   | GET      | `/v2/`                                                       | 200     | 404/401     | X    |
| end-2   | GET/HEAD | `/v2/<name>/blobs/<digest>`                                  | 200     | 404         | X    |
| end-3   | GET/HEAD | `/v2/<name>/manifests/<reference>`                           | 200     | 404         |      |
| end-4a  | POST     | `/v2/<name>/blobs/uploads/`                                  | 202     | 404         | X    |
| end-4b  | POST     | `/v2/<name>/blobs/uploads/?digest=<digest>`                  | 201/202 | 404/400     | X    |
| end-5   | PATCH    | `/v2/<name>/blobs/uploads/<reference>`                       | 202     | 404/416     | X    |
| end-6   | PUT      | `/v2/<name>/blobs/uploads/<reference>?digest=<digest>`       | 201     | 404/400     | X    |
| end-7   | PUT      | `/v2/<name>/manifests/<reference>`                           | 201     | 404         | X    |
| end-8a  | GET      | `/v2/<name>/tags/list`                                       | 200     | 404         |      |
| end-8b  | GET      | `/v2/<name>/tags/list?n=<integer>&last=<integer>`            | 200     | 404         |      |
| end-9   | DELETE   | `/v2/<name>/manifests/<reference>`                           | 202     | 404/400/405 |      |
| end-10  | DELETE   | `/v2/<name>/blobs/<digest>`                                  | 202     | 404/405     |      |
| end-11  | POST     | `/v2/<name>/blobs/uploads/?mount=<digest>&from=<other_name>` | 201     | 404         |      |
| end-12a | GET      | `/v2/<name>/referrers/<digest>`                              | 200     | 404/400     |      |
| end-12b | GET      | `/v2/<name>/referrers/<digest>?artifactType=<artifactType>`  | 200     | 404/400     |      |
| end-13  | GET      | `/v2/<name>/blobs/uploads/<reference>`                       | 204     | 404         |      |

https://specs.opencontainers.org/distribution-spec/#endpoints
