use anyhow::Result;
use serde::{Deserialize, Serialize};

mod image_index;
mod image_manifest;
mod tags_list;

pub use image_index::ImageIndex;
pub use image_manifest::ImageManifest;
pub use tags_list::TagsList;

/// Intermediate struct to validate the manifest data. The media type is used to
/// determine the manifest variant that should be used for validation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde_with::skip_serializing_none]
pub struct UnknownSchema {
  pub media_type: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum SchemaVariant {
  ImageManifest(Box<ImageManifest>),
  ImageIndex(Box<ImageIndex>),
  Unknown(Box<UnknownSchema>),
}

pub fn validate_manifest_data(
  data: Vec<u8>,
  content_type: Option<String>,
) -> Result<SchemaVariant> {
  let unknown = match serde_json::from_slice::<UnknownSchema>(&data) {
    Ok(unknown) => unknown,
    Err(e) => return Err(anyhow::anyhow!("Invalid manifest data: {:?}", e)),
  };

  let media_type = match unknown.media_type.clone() {
    Some(media_type) => {
      if let Some(content_type) = content_type {
        // If a manifest includes a mediaType field, clients MUST set the
        // Content-Type header to the value specified by the mediaType field.
        if content_type != media_type {
          return Err(anyhow::anyhow!(
            "Content type mismatch: Was '{media_type}' internally, got '{content_type}'",
          ));
        }
      }
      media_type
    }
    None => {
      if let Some(content_type) = content_type {
        content_type
      } else {
        return Err(anyhow::anyhow!("No media type provided"));
      }
    }
  };

  // Image manifest
  // - application/vnd.oci.image.manifest.v1+json
  // - application/vnd.docker.distribution.manifest.v2+json
  // Image index
  // - application/vnd.oci.image.index.v1+json
  // - application/vnd.docker.distribution.manifest.list.v2+json
  // https://github.com/opencontainers/image-spec/blob/main/media-types.md#compatibility-matrix
  match media_type.as_str() {
    "application/vnd.oci.image.manifest.v1+json" => parse_image_manifest(&data),
    "application/vnd.docker.distribution.manifest.v2+json" => parse_image_manifest(&data),
    "application/vnd.oci.image.index.v1+json" => parse_image_index(&data),
    "application/vnd.docker.distribution.manifest.list.v2+json" => parse_image_index(&data),
    _ => Ok(SchemaVariant::Unknown(Box::new(unknown))),
  }
}

fn parse_image_manifest(data: &[u8]) -> Result<SchemaVariant> {
  match serde_json::from_slice::<image_manifest::ImageManifest>(data) {
    Ok(manifest) => Ok(SchemaVariant::ImageManifest(Box::new(manifest))),
    Err(e) => Err(anyhow::anyhow!("Invalid manifest data: {:?}", e)),
  }
}

fn parse_image_index(data: &[u8]) -> Result<SchemaVariant> {
  match serde_json::from_slice::<image_index::ImageIndex>(data) {
    Ok(index) => Ok(SchemaVariant::ImageIndex(Box::new(index))),
    Err(e) => Err(anyhow::anyhow!("Invalid manifest data: {:?}", e)),
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::schemas::{image_index::ImageIndexMediaType, image_manifest::ImageManifestMediaType};

  #[test]
  fn test_validate_manifest_data() {
    let data = validate_manifest_data(
      "{\"mediaType\": \"application/vnd.oci.unknown.v1+json\", \"test\": 2}".as_bytes().to_vec(),
      None,
    );
    match data.unwrap() {
      SchemaVariant::Unknown(base) => {
        assert_eq!(base.media_type, Some("application/vnd.oci.unknown.v1+json".to_string()));
      }
      _ => panic!("Expected BaseManifest variant"),
    }

    let manifest_json = include_str!("../tests/fixtures/image_manifest.json");
    let data = validate_manifest_data(manifest_json.as_bytes().to_vec(), None);
    match data.unwrap() {
      SchemaVariant::ImageManifest(manifest) => {
        assert_eq!(manifest.media_type, Some(ImageManifestMediaType::DockerManifestV2));
      }
      _ => panic!("Expected ImageManifest variant"),
    }

    let index_json = include_str!("../tests/fixtures/image_index.json");
    let data = validate_manifest_data(
      index_json.as_bytes().to_vec(),
      Some("application/vnd.oci.image.index.v1+json".to_string()),
    );
    match data.unwrap() {
      SchemaVariant::ImageIndex(index) => {
        assert_eq!(index.media_type, Some(ImageIndexMediaType::OCIImageIndexV1));
      }
      _ => panic!("Expected ImageIndex variant"),
    }

    let data = validate_manifest_data(
      index_json.as_bytes().to_vec(),
      Some("application/unsupported.media.type+json".to_string()),
    );
    assert!(data.is_err());
  }

  #[test]
  fn test_parse_image_manifest() {
    let manifest_json = include_str!("../tests/fixtures/image_manifest.json");
    let manifest_variant = parse_image_manifest(manifest_json.as_bytes()).unwrap();
    match manifest_variant {
      SchemaVariant::ImageManifest(manifest) => {
        assert_eq!(manifest.media_type, Some(ImageManifestMediaType::DockerManifestV2));
      }
      _ => panic!("Expected ImageManifest variant"),
    }
  }

  #[test]
  fn test_parse_image_index() {
    let index_json = include_str!("../tests/fixtures/image_index.json");
    let index_variant = parse_image_index(index_json.as_bytes()).unwrap();
    match index_variant {
      SchemaVariant::ImageIndex(index) => {
        assert_eq!(index.media_type, Some(ImageIndexMediaType::OCIImageIndexV1));
      }
      _ => panic!("Expected ImageIndex variant"),
    }
  }
}
