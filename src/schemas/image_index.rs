use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// For the media type that this document is compatible with, see the matrix.
/// <https://github.com/opencontainers/image-spec/blob/main/media-types.md#compatibility-matrix>
///
/// Compatibility notes:
/// - `.annotations`: only present in OCI
/// - `.[]manifests.annotations`: only present in OCI
/// - `.[]manifests.urls`: only present in OCI
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageIndexMediaType {
  #[serde(rename = "application/vnd.oci.image.index.v1+json")]
  OCIImageIndexV1,
  #[serde(rename = "application/vnd.docker.distribution.manifest.list.v2+json")]
  DockerManifestListV2,
}

/// The image index is a higher-level manifest which points to specific image
/// manifests, ideal for one or more platforms. While the use of an image index
/// is OPTIONAL for image providers, image consumers SHOULD be prepared to
/// process them.
///
/// This defines the `application/vnd.oci.image.index.v1+json` media type.
///
/// <https://github.com/opencontainers/image-spec/blob/v1.0.1/image-index.md>
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct ImageIndex {
  /// This REQUIRED property specifies the image manifest schema version. For
  /// this version of the specification, this MUST be 2 to ensure backward
  /// compatibility with older versions of Docker. The value of this field will
  /// not change. This field MAY be removed in a future version of the
  /// specification.
  pub schema_version: u32,

  /// This property is reserved for use, to maintain compatibility. When used,
  /// this field contains the media type of this document, which differs from
  /// the descriptor use of mediaType.
  pub media_type: Option<ImageIndexMediaType>,

  /// This REQUIRED property contains a list of manifests for specific
  /// platforms. While this property MUST be present, the size of the array
  /// MAY be zero.
  pub manifests: Vec<Manifest>,
}

/// Each object in manifests includes a set of descriptor properties
/// with some additional properties and restrictions.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct Manifest {
  /// This REQUIRED property contains the media type of the referenced content.
  /// Values MUST comply with RFC 6838, including the naming requirements in
  /// its section 4.2.
  ///
  /// <https://datatracker.ietf.org/doc/html/rfc6838#section-4.2>
  ///
  /// This descriptor property has additional restrictions for manifests.
  /// Implementations MUST support at least the following media types:
  /// - application/vnd.oci.image.manifest.v1+json
  pub media_type: String,

  /// This REQUIRED property is the digest of the targeted content, conforming
  /// to the requirements outlined in Digests. Retrieved content SHOULD be
  /// verified against this digest when consumed via untrusted sources.
  pub digest: String,

  /// This REQUIRED property specifies the size, in bytes, of the raw content.
  /// This property exists so that a client will have an expected size for the
  /// content before processing. If the length of the retrieved content does not
  /// match the specified length, the content SHOULD NOT be trusted.
  pub size: u64,

  /// This OPTIONAL property specifies a list of URIs from which this object MAY
  /// be downloaded. Each entry MUST conform to RFC 3986. Entries SHOULD use the
  /// http and https schemes, as defined in RFC 7230.
  ///
  /// <https://datatracker.ietf.org/doc/html/rfc3986>
  /// <https://datatracker.ietf.org/doc/html/rfc7230>
  pub urls: Option<Vec<String>>,

  /// This OPTIONAL property contains arbitrary metadata for this descriptor.
  pub annotations: Option<Annotations>,

  /// This OPTIONAL property describes the minimum runtime requirements of the
  /// image. This property SHOULD be present if its target is platform-specific.
  pub platform: Option<Platform>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct Platform {
  /// This REQUIRED property specifies the CPU architecture. Image indexes
  /// SHOULD use, and implementations SHOULD understand, values listed in the
  /// Go Language document for GOARCH.
  ///
  /// <https://go.dev/doc/install/source#environment>
  pub architecture: String,

  /// This REQUIRED property specifies the operating system. Image indexes
  /// SHOULD use, and implementations SHOULD understand, values listed in the Go
  /// Language document for GOOS.
  ///
  /// <https://go.dev/doc/install/source#environment>
  pub os: String,

  /// This OPTIONAL property specifies the version of the operating system
  /// targeted by the referenced blob. Implementations MAY refuse to use
  /// manifests where os.version is not known to work with the host OS version.
  /// Valid values are implementation-defined. e.g. 10.0.14393.1066 on windows.
  #[serde(rename = "os.version")]
  pub os_version: Option<String>,

  /// This OPTIONAL property specifies an array of strings, each specifying a
  /// mandatory OS feature. When os is windows, image indexes SHOULD use,
  /// and implementations SHOULD understand the following values:
  /// - win32k: image requires win32k.sys on the host
  ///
  /// When os is not windows, values are implementation-defined and SHOULD be
  /// submitted to this specification for standardization.
  #[serde(rename = "os.features")]
  pub os_features: Option<Vec<String>>,

  /// This OPTIONAL property specifies the variant of the CPU. Image indexes
  /// SHOULD use, and implementations SHOULD understand, values listed in the
  /// following table. When the variant of the CPU is not listed in the table,
  /// values are implementation-defined and SHOULD be submitted to this
  /// specification for standardization.
  ///
  /// | ISA/ABI        | architecture | variant |
  /// |----------------|--------------|---------|
  /// | ARM 32-bit, v6 | arm          | v6      |
  /// | ARM 32-bit, v7 | arm          | v7      |
  /// | ARM 32-bit, v8 | arm          | v8      |
  /// | ARM 64-bit, v8 | arm64        | v8      |
  pub variant: Option<String>,

  /// This property is RESERVED for future versions of the specification.
  pub features: Option<Vec<String>>,
}

/// Annotations MUST be a key-value map where both the key and value MUST be
/// strings.
///
/// <https://github.com/opencontainers/image-spec/blob/v1.0.1/annotations.md#rules>
pub type Annotations = HashMap<String, String>;
