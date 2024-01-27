# OCI distributor

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
