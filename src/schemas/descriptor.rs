use crate::schemas::annotations::Annotations;
use serde::{Deserialize, Serialize};

/// <https://github.com/opencontainers/image-spec/blob/v1.0.1/descriptor.md>
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct Descriptor {
  /// This REQUIRED property contains the media type of the referenced content.
  /// Values MUST comply with RFC 6838, including the naming requirements in
  /// its section 4.2.
  ///
  /// <https://datatracker.ietf.org/doc/html/rfc6838#section-4.2>
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
}
