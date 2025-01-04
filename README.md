# Container Cubby

Container Cubby is a minimal implementation of a container registry, based on the [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md). It is single-tenant, and stores all container data in a local directory. Although it does work and implements most of the spec, there are no guarantees about the stability or security, and there might still be frequent breaking changes.

## Storage

> [!NOTE]
> Since symlinks and extended attributes are OS-specific features, Container Cubby is not guaranteed to work on all operating systems. It is periodically tested and expected to work on MacOS, Ubuntu and Arch Linux. To verify that your OS is supported, run `cargo test` and check that all tests pass.
>
> Feel free to [open an issue](https://github.com/rosvik/container-cubby/issues/new) if you find that your OS is not supported.

While the most common way for an API to store data is trough a traditional database using SQL and a blob storage for larger files, Container Cubby uses a different approach. Container Cubby stores all container data in a local directory. The location of this directory can be set by the `DATA_DIR` environment variable.

This is done by using
- directories to represent namespaces.
  - E.g. containers under the `rosvik/container-cubby` namespace are stored in `<DATA_DIR>/containers/rosvik/container-cubby/`.
- files to store manifest and blob data.
  - Manifests are stored as `sha256@<manifest hash>.json` in namespace folders.
  - Blobs are stored in the `<DATA_DIR>/blobs` folder.
- symbolic links to represent relations between files.
  - Tags are represented as symlinks to manifest files.
  - Instead of storing blobs per namespace, namespace directories has symlinks to the blob directory. This way, if two namespaces point to the same blob hash, the same blob file is used.
- [extended attributes](https://wiki.archlinux.org/title/Extended_attributes) to store file metadata.
  - The `user.mime_type` extended attribute is used to store the media type of manifests.

As an example, if the image `foo/bar:latest` is created with a manifest and one blob with hash `sha256@abc123`, then the following four files will be created:

```
<DATA_DIR>/
├── containers/
│   └── foo/
│       └── bar/
│           ├── latest.json                      [SYMLINK]
│           ├── sha256@<manifest hash>.json
│           └── sha256@abc123.blob               [SYMLINK]
└── blobs/
    └── ab/
        └── c123.blob
```

Where `latest.json` is a symlink to `sha256@<manifest hash>.json`, and `sha256@abc123.blob` is a symlink to `blobs/ab/c123.blob`.

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
| end-8a  | GET      | `/v2/<name>/tags/list`                                       | 200     | 404         | X    |
| end-8b  | GET      | `/v2/<name>/tags/list?n=<integer>&last=<tagname>`            | 200     | 404         | X    |
| end-9   | DELETE   | `/v2/<name>/manifests/<reference>`                           | 202     | 404/400/405 | X    |
| end-10  | DELETE   | `/v2/<name>/blobs/<digest>`                                  | 202     | 404/405     | X    |
| end-11  | POST     | `/v2/<name>/blobs/uploads/?mount=<digest>&from=<other_name>` | 201     | 404         | X    |
| end-12a | GET      | `/v2/<name>/referrers/<digest>`                              | 200     | 404/400     |      |
| end-12b | GET      | `/v2/<name>/referrers/<digest>?artifactType=<artifactType>`  | 200     | 404/400     |      |
| end-13  | GET      | `/v2/<name>/blobs/uploads/<reference>`                       | 204     | 404         | X    |

https://specs.opencontainers.org/distribution-spec/#endpoints

## Spec links

- [OCI Image Format Specification](https://github.com/opencontainers/image-spec)
- [OCI Runtime Specification](https://github.com/opencontainers/runtime-spec)
- [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec)
  - [1.1 Release notes](https://opencontainers.org/posts/blog/2024-03-13-image-and-distribution-1-1/)
- [CNCF Token Authentication Specification](https://distribution.github.io/distribution/spec/auth/token/)

## Related projects

- [Azure Container Registry Documentation](https://learn.microsoft.com/en-us/rest/api/containerregistry)
- [google/go-containerregistry Documentation](https://github.com/google/go-containerregistry/blob/main/pkg/v1/remote/README.md)
- [CNCF Distribution Reference](https://distribution.github.io/distribution/spec/)
- [skopeo](https://github.com/containers/skopeo)
- [Cloudflare Container Registry in Workers](https://github.com/cloudflare/serverless-registry)

### Rust Implementations

- [mcronce/oci-registry](https://github.com/mcronce/oci-registry)
- [krustlet/oci-distribution](https://github.com/krustlet/oci-distribution)

### Go Implementations

- [distribution/distribution](https://github.com/distribution/distribution/)
- [google/go-containerregistry/pkg/registry](https://github.com/google/go-containerregistry/blob/main/pkg/registry/README.md)
