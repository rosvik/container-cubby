use serde::{Deserialize, Serialize};
use serde_json::Result;

mod image_manifest;

pub use image_manifest::ImageManifest;

/// Intermediate struct to validate the manifest data. The media type is used to
/// determine the manifest variant that should be used for validation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct BaseManifest {
  pub media_type: String,
}

#[derive(Debug)]
pub enum ManifestVariant {
  ImageManifest(ImageManifest),
  BaseManifest(BaseManifest),
}

pub fn validate_manifest_data(data: Vec<u8>) -> Result<ManifestVariant> {
  let base = match serde_json::from_slice::<BaseManifest>(&data) {
    Ok(base) => base,
    Err(e) => return Err(e),
  };

  // Image manifest
  // - application/vnd.oci.image.manifest.v1+json
  // - application/vnd.docker.distribution.manifest.v2+json
  // https://github.com/opencontainers/image-spec/blob/main/media-types.md#compatibility-matrix
  match base.media_type.as_str() {
    "application/vnd.oci.image.manifest.v1+json" => parse_image_manifest(&data),
    "application/vnd.docker.distribution.manifest.v2+json" => parse_image_manifest(&data),
    _ => Ok(ManifestVariant::BaseManifest(base)),
  }
}

fn parse_image_manifest(data: &[u8]) -> Result<ManifestVariant> {
  match serde_json::from_slice::<image_manifest::ImageManifest>(data) {
    Ok(manifest) => Ok(ManifestVariant::ImageManifest(manifest)),
    Err(e) => Err(e),
  }
}
#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_validate_manifest_data() {
    let data = validate_manifest_data(
      "{\"mediaType\": \"application/vnd.oci.unknown.v1+json\", \"test\": 2}".as_bytes().to_vec(),
    );
    match data.unwrap() {
      ManifestVariant::BaseManifest(base) => {
        assert_eq!(base.media_type, "application/vnd.oci.unknown.v1+json".to_string());
      }
      _ => panic!("Expected BaseManifest variant"),
    }
  }

  #[test]
  fn test_parse_image_manifest() {
    let manifest_json = include_str!("../tests/fixtures/manifest.json");
    let manifest_variant = parse_image_manifest(manifest_json.as_bytes()).unwrap();
    match manifest_variant {
      ManifestVariant::ImageManifest(manifest) => {
        assert_eq!(
          manifest.media_type,
          Some("application/vnd.docker.distribution.manifest.v2+json".to_string())
        );
      }
      _ => panic!("Expected ImageManifest variant"),
    }
  }
}
