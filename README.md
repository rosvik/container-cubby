# Container Cubby

> [!WARNING]
> Container Cubby is still in development, and may be subject to frequent and unannounced breaking changes.

Container Cubby is a container registry that aims to implement the [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md) while avoiding complex features and dependencies. It is single-tenant, and stores all container data as files in a local directory.

## Running

### As a container

Pre-built images are available from [cubby.no](https://cubby.no) (Which, of course, runs Container Cubby ✨).

```bash
podman pull cubby.no/rosvik/container-cubby:main
```

```bash
podman run -d --name container-cubby -p 8602:8602 \
  -e HOST=0.0.0.0 \
  -e AUTH_MODE=write_only \
  -e USERNAME=admin \
  -e PASSWORD=hunter2 \
  cubby.no/rosvik/container-cubby:main
```

### Building from source

[Install rust](https://rustup.rs/), then execute the following to create and run an optimized binary:

```bash
cargo build --release && ./target/release/container-cubby
```

## Environment variables

| Variable     | Description                                     | Default     |
| ------------ | ----------------------------------------------- | ---------   |
| `HOST`       | The host to bind to.                            | `localhost` |
| `PORT`       | The port to bind to.                            | `8602`      |
| `DATA_DIR`   | The directory to store the container data in.   | `./data`    |
| `USERNAME`   | The username to use for authentication.         |             |
| `PASSWORD`   | The password to use for authentication.         |             |
| `AUTH_MODE`  | The authentication mode to use. `none`, `write_only` or `read_write`.  | Required |
| `PRUNE_CRON` | A cron expression that sets the schedule for pruning the database. | Disabled |

See [`.env.example`](.env.example) for a starting point.

### `AUTH_MODE`

Container Cubby implements the following authentication modes:

- `none`: No authentication is required for reading or writing. Not recommended if the registry is publicly accessible.
- `write_only`: Authentication is required for writing, but not for reading.
- `read_write`: Authentication is required for both reading and writing.

The only type of authentication that is supported is basic auth for a single user/password pair. If no username or password is provided, the registry will be set to read-only mode.

### `PRUNE_CRON`

To avoid bloating the database with unused data, Container Cubby can be configured to delete untagged images. Images that are less than 24 hours old are not deleted to avoid issues with ongoing uploads.

Pruning is disabled by default. It is enabled by setting `PRUNE_CRON` as a [cron](https://en.wikipedia.org/wiki/Cron) expression. The following example will prune the database every day at midnight UTC:

```bash
# sec, min, hour, day of month, month, day of week
PRUNE_CRON="0 0 0 * * *"
```

## Storage

Container Cubby stores all container data in a local directory. The location of this directory can be set by the `DATA_DIR` environment variable.

This is implemented using:
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

As an example, if the image `rosvik/container-cubby:latest` is created as one manifest which points to two blobs with hashes `sha256:abcdef123456` and `sha256:123456abcdef`, then the following files will be created:

```
<DATA_DIR>/
├── containers/
│   └── rosvik/
│       └── container-cubby/
│           ├── latest.json                      [SYMLINK]
│           ├── sha256:<manifest hash>.json
│           ├── sha256:abcdef123456.blob         [SYMLINK]
│           └── sha256:123456abcdef.blob         [SYMLINK]
└── blobs/
    ├── 12/
    │   └── 3456abcdef.blob
    └── ab/
        └── cdef123456.blob
```

Here, `latest.json` is a symlink to `sha256:<manifest hash>.json`, and `sha256:abcdef123456.blob` is a symlink to `blobs/ab/cdef123456.blob`.

> [!NOTE]
> Since symlinks and extended attributes are OS-specific features, Container Cubby is not guaranteed to work on all operating systems. It is periodically tested and expected to work on MacOS, Debian, Ubuntu and Arch Linux. To verify that your OS is supported, run `cargo test` and check that all tests pass.

### Handling of the data directory

Due to the use of symlinks and extended attributes, some caveats apply when you want to move or backup the data directory. For example, the `cp` command doesn't preserve extended attributes, and the `--archive` parameter should be used to preserve both symlinks and extended attributes.

```bash
cp --archive data/ data-backup/
```

You can read more about this on ArchWiki under [Extended Attributes > Preserving extended attributes](https://wiki.archlinux.org/title/Extended_attributes#Preserving_extended_attributes).

## Spec conformance

The endpoints as defined by the [OCI Distribution Spec](https://specs.opencontainers.org/distribution-spec/#endpoints), and the project's current progress is as follows:

| ID      | Method   | API Endpoint                                                 | Implemented |
| ------- | -------- | ------------------------------------------------------------ | ----------- |
| end-1   | GET      | `/v2/`                                                       | ✅          |
| end-2   | GET/HEAD | `/v2/<name>/blobs/<digest>`                                  | ✅          |
| end-3   | GET/HEAD | `/v2/<name>/manifests/<reference>`                           | ✅          |
| end-4a  | POST     | `/v2/<name>/blobs/uploads/`                                  | ✅          |
| end-4b  | POST     | `/v2/<name>/blobs/uploads/?digest=<digest>`                  | ✅          |
| end-5   | PATCH    | `/v2/<name>/blobs/uploads/<reference>`                       | ✅          |
| end-6   | PUT      | `/v2/<name>/blobs/uploads/<reference>?digest=<digest>`       | ✅          |
| end-7   | PUT      | `/v2/<name>/manifests/<reference>`                           | ✅          |
| end-8a  | GET      | `/v2/<name>/tags/list`                                       | ✅          |
| end-8b  | GET      | `/v2/<name>/tags/list?n=<integer>&last=<tagname>`            | ✅          |
| end-9   | DELETE   | `/v2/<name>/manifests/<reference>`                           | ✅          |
| end-10  | DELETE   | `/v2/<name>/blobs/<digest>`                                  | ✅          |
| end-11  | POST     | `/v2/<name>/blobs/uploads/?mount=<digest>&from=<other_name>` | ✅          |
| end-12a | GET      | `/v2/<name>/referrers/<digest>`                              | ❌          |
| end-12b | GET      | `/v2/<name>/referrers/<digest>?artifactType=<artifactType>`  | ❌          |
| end-13  | GET      | `/v2/<name>/blobs/uploads/<reference>`                       | ✅          |

The referrers endpoints (end-12a and end-12b) are not implemented, mostly due to there not being an obvious way to support it without slowly reading all manifests per namespace on disk. See [this issue](https://github.com/rosvik/container-cubby/issues/23) for progress on this when/if a solution is being looked into.

This means the registry is not fully conformant with v1.1 of the spec. Conformance with v1.0 should be quite good, but see my fork of the spec conformance test suite for some nitpicks: [rosvik/oci-distribution-spec](https://github.com/rosvik/oci-distribution-spec)

That being said, there are plenty of details around pulling and pushing images that tools rely on that are not covered by the spec. So as a secondary goal, Container Cubby will try to support [Podman](https://podman.io/), [Skopeo](https://github.com/containers/skopeo), [Docker CLI](https://www.docker.com/products/cli/) and [Containerization](https://github.com/apple/containerization).

If you find any issues with these tools or spec compliance, feel free to open an issue!

## Contributing

Contributions are very welcome! If you notice any bugs or have suggestions, please feel free to open an issue or make a PR.
