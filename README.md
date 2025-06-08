# Container Cubby

> [!WARNING]
> Container Cubby is currently in development, and may still be subject to frequent and unannounced breaking changes.

Container Cubby is a minimal implementation of a container registry, based on the [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md). It is single-tenant, and stores all container data in a local directory. Although it does work and implements most of the spec, there are no guarantees about the stability or security, and there might still be frequent breaking changes.

## Storage

While the most common way for an API to store data is trough a traditional database using SQL and a blob storage for larger files, Container Cubby uses a different approach. Container Cubby stores all container data in a local directory. The location of this directory can be set by the `DATA_DIR` environment variable.

This is implemented using
- directories to represent namespaces.
  - E.g. containers under the `rosvik/container-cubby` namespace are stored in `<DATA_DIR>/containers/rosvik/container-cubby/`.
- files to store manifest and blob data.
  - Manifests are stored as `sha256:<manifest hash>.json` in namespace folders.
  - Blobs are stored in the `<DATA_DIR>/blobs` folder.
- symbolic links to represent relations between files.
  - Tags are represented as symlinks to manifest files.
  - Instead of storing blobs per namespace, the namespace directories contains symlinks to the blob directory. This way, when two namespaces refers to blobs with the same hash, only one blob file is stored.
- [extended attributes](https://wiki.archlinux.org/title/Extended_attributes) to store file metadata.
  - The `user.mime_type` extended attribute is used to store the media type of manifests.

As an example, if the image `rosvik/container-cubby:latest` is created as one manifest and one blob with hash `sha256:abc123`, then the following files will be created:

```
<DATA_DIR>/
├── containers/
│   └── rosvik/
│       └── container-cubby/
│           ├── latest.json                      [SYMLINK]
│           ├── sha256:<manifest hash>.json
│           └── sha256:abc123.blob               [SYMLINK]
└── blobs/
    └── ab/
        └── c123.blob
```

Here, `latest.json` is a symlink to `sha256:<manifest hash>.json`, and `sha256:abc123.blob` is a symlink to `blobs/ab/c123.blob`.

> [!NOTE]
> Since symlinks and extended attributes are OS-specific features, Container Cubby is not guaranteed to work on all operating systems. It is periodically tested and expected to work on MacOS, Debian, Ubuntu and Arch Linux. To verify that your OS is supported, run `cargo test` and check that all tests pass.

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

## Contributing

Contributions are very welcome! If you notice any bugs, or have any suggestions, please feel free to open an issue or make a PR.
