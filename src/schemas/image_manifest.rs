use serde::{Deserialize, Serialize};

use crate::schemas::{annotations::Annotations, descriptor::Descriptor};

/// For the media type that this document is compatible with, see the matrix.
/// <https://github.com/opencontainers/image-spec/blob/main/media-types.md#compatibility-matrix>
///
/// Compatibility notes:
/// - `.annotations`: only present in OCI
/// - `.config.annotations`: only present in OCI
/// - `.config.urls`: only present in OCI
/// - `.[]layers.annotations`: only present in OCI
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageManifestMediaType {
  #[serde(rename = "application/vnd.oci.image.manifest.v1+json")]
  OCIImageManifestV1,
  #[serde(rename = "application/vnd.docker.distribution.manifest.v2+json")]
  DockerManifestV2,
}

/// An image manifest provides a configuration and set of layers for a single
/// container image for a specific architecture and operating system.
///
/// This describes the `application/vnd.oci.descriptor.v1+json` media type.
///
/// <https://github.com/opencontainers/image-spec/blob/v1.0.1/manifest.md#image-manifest>
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct ImageManifest {
  /// This REQUIRED property specifies the image manifest schema version. For
  /// this version of the specification, this MUST be 2 to ensure backward
  /// compatibility with older versions of Docker. The value of this field will
  /// not change. This field MAY be removed in a future version of the
  /// specification.
  pub schema_version: u8,

  /// This property is reserved for use, to maintain compatibility. When used,
  /// this field contains the media type of this document, which differs from
  /// the descriptor use of mediaType.
  pub media_type: Option<ImageManifestMediaType>,

  /// This REQUIRED property references a configuration object for a container,
  /// by digest.
  ///
  /// <https://github.com/opencontainers/image-spec/blob/v1.0.1/descriptor.md>
  pub config: Descriptor,

  /// Each item in the array MUST be a descriptor. The array MUST have the base
  /// layer at index 0. Subsequent layers MUST then follow in stack order (i.e.
  /// from `layers[0]` to `layers[len(layers)-1]`). The final filesystem layout
  /// MUST match the result of applying the layers to an empty directory. The
  /// ownership, mode, and other attributes of the initial empty directory are
  /// unspecified.
  pub layers: Vec<Descriptor>,

  /// This OPTIONAL property specifies a descriptor of another manifest. This
  /// value defines a weak association to a separate Merkle Directed Acyclic
  /// Graph (DAG) structure, and is used by the referrers API to include this
  /// manifest in the list of responses for the subject digest.
  ///
  /// <https://en.wikipedia.org/wiki/Merkle_tree>
  pub subject: Option<Descriptor>,

  /// This OPTIONAL property contains arbitrary metadata for the image manifest.
  pub annotations: Option<Annotations>,
}
