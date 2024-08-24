use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An image manifest provides a configuration and set of layers for a single
/// container image for a specific architecture and operating system.
///
/// This describes the `application/vnd.oci.descriptor.v1+json` media type.
///
/// https://github.com/opencontainers/image-spec/blob/v1.0.1/manifest.md#image-manifest
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
  /// This REQUIRED property specifies the image manifest schema version. For
  /// this version of the specification, this MUST be 2 to ensure backward
  /// compatibility with older versions of Docker. The value of this field will
  /// not change. This field MAY be removed in a future version of the
  /// specification.
  #[serde(rename = "schemaVersion")]
  pub schema_version: u32,

  /// This property is reserved for use, to maintain compatibility. When used,
  /// this field contains the media type of this document, which differs from
  /// the descriptor use of mediaType.
  #[serde(rename = "mediaType")]
  pub media_type: String,

  /// This REQUIRED property references a configuration object for a container,
  /// by digest.
  ///
  /// <https://github.com/opencontainers/image-spec/blob/v1.0.1/descriptor.md>
  pub config: Descriptor,

  /// Each item in the array MUST be a descriptor. The array MUST have the base
  /// layer at index 0. Subsequent layers MUST then follow in stack order (i.e.
  /// from layers[0] to layers[len(layers)-1]). The final filesystem layout MUST
  /// match the result of applying the layers to an empty directory. The
  /// ownership, mode, and other attributes of the initial empty directory are
  /// unspecified.
  pub layers: Vec<Descriptor>,

  /// This OPTIONAL property contains arbitrary metadata for the image manifest.
  pub annotations: Option<Annotations>,
}

/// <https://github.com/opencontainers/image-spec/blob/v1.0.1/descriptor.md>
#[derive(Debug, Serialize, Deserialize)]
pub struct Descriptor {
  /// This REQUIRED property contains the media type of the referenced content.
  /// Values MUST comply with RFC 6838, including the naming requirements in
  /// its section 4.2.
  ///
  /// <https://datatracker.ietf.org/doc/html/rfc6838#section-4.2>
  #[serde(rename = "mediaType")]
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
  pub urls: Vec<String>,

  /// This OPTIONAL property contains arbitrary metadata for this descriptor.
  pub annotations: Option<Annotations>,
}

/// Annotations MUST be a key-value map where both the key and value MUST be
/// strings.
///
/// <https://github.com/opencontainers/image-spec/blob/v1.0.1/annotations.md#rules>
pub type Annotations = HashMap<String, String>;
