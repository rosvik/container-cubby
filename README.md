# tiny-registry

This is (an attempt at creating) a minimal implementation of a container registry, that closely follows the [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md). The project is still a work in progress, but the goal is to create a no-fuss single-tenant container registry in rust.

## TODO

- [ ] Use postgres
- [ ] Add testcontainers for bootstraping and testing
- [ ] Implement the endpoints
- [ ] Use a proper blob storage backend
  - S3?
  - Some other self hosted blob storage?
  - Files on disk?

## Endpoints

The endpoints defined by the spec, and the project's current progress is the following:

| ID      | Method   | API Endpoint                                                 | Success | Failure     | Todo |
| ------- | -------- | ------------------------------------------------------------ | ------- | ----------- | ---- |
| end-1   | GET      | `/v2/`                                                       | 200     | 404/401     | X    |
| end-2   | GET/HEAD | `/v2/<name>/blobs/<digest>`                                  | 200     | 404         | X    |
| end-3   | GET/HEAD | `/v2/<name>/manifests/<reference>`                           | 200     | 404         | X    |
| end-4a  | POST     | `/v2/<name>/blobs/uploads/`                                  | 202     | 404         | X    |
| end-4b  | POST     | `/v2/<name>/blobs/uploads/?digest=<digest>`                  | 201/202 | 404/400     | X    |
| end-5   | PATCH    | `/v2/<name>/blobs/uploads/<reference>`                       | 202     | 404/416     | X    |
| end-6   | PUT      | `/v2/<name>/blobs/uploads/<reference>?digest=<digest>`       | 201     | 404/400     | X    |
| end-7   | PUT      | `/v2/<name>/manifests/<reference>`                           | 201     | 404         | X    |
| end-8a  | GET      | `/v2/<name>/tags/list`                                       | 200     | 404         | -    |
| end-8b  | GET      | `/v2/<name>/tags/list?n=<integer>&last=<integer>`            | 200     | 404         | -    |
| end-9   | DELETE   | `/v2/<name>/manifests/<reference>`                           | 202     | 404/400/405 | X    |
| end-10  | DELETE   | `/v2/<name>/blobs/<digest>`                                  | 202     | 404/405     | X    |
| end-11  | POST     | `/v2/<name>/blobs/uploads/?mount=<digest>&from=<other_name>` | 201     | 404         |      |
| end-12a | GET      | `/v2/<name>/referrers/<digest>`                              | 200     | 404/400     |      |
| end-12b | GET      | `/v2/<name>/referrers/<digest>?artifactType=<artifactType>`  | 200     | 404/400     |      |
| end-13  | GET      | `/v2/<name>/blobs/uploads/<reference>`                       | 204     | 404         | X    |

https://specs.opencontainers.org/distribution-spec/#endpoints

## Spec links

- [OCI Image Format Specification](https://github.com/opencontainers/image-spec)
- [OCI Runtime Specification](https://github.com/opencontainers/runtime-spec)
- [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec)
  - [1.1 Release notes](https://opencontainers.org/posts/blog/2024-03-13-image-and-distribution-1-1/)

## Related projects

- [Azure Container Registry Documentation](https://learn.microsoft.com/en-us/rest/api/containerregistry)
- [google/go-containerregistry Documentation](https://github.com/google/go-containerregistry/blob/main/pkg/v1/remote/README.md)
- [CNCF Distribution Reference](https://distribution.github.io/distribution/spec/)
- [skopeo](https://github.com/containers/skopeo)

### Rust Implementations

- [mcronce/oci-registry](https://github.com/mcronce/oci-registry)
- [krustlet/oci-distribution](https://github.com/krustlet/oci-distribution)

### Go Implementations

- [distribution/distribution](https://github.com/distribution/distribution/)
- [google/go-containerregistry/pkg/registry](https://github.com/google/go-containerregistry/blob/main/pkg/registry/README.md)
