use crate::{
  digestor,
  storage::{ensure_container_dir_exists, file, path, symlink, xattr::set_xattr_media_type},
  utils::{is_safe_name, verify_reference, DigestMismatch, Reference},
};
use std::io::{self, Write};

pub struct Manifest {
  pub name: String,
  pub reference: Reference,
}

impl Manifest {
  pub fn new(name: &str, reference: &str) -> Result<Self, io::Error> {
    if !is_safe_name(name) {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid name"));
    }

    let reference = verify_reference(reference.to_string())
      .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid reference"))?;

    Ok(Self {
      name: name.to_string(),
      reference,
    })
  }

  /// Creates the manifest file in the container directory, verifies the digest,
  /// and creates a tag symlink if the reference is a tag.
  pub fn create_manifest(
    &self,
    data: Vec<u8>,
    content_type: Option<String>,
  ) -> Result<(), io::Error> {
    ensure_container_dir_exists(&self.name)?;

    // Verify that the digest matches the data
    let digest = self
      .verified_digest(&data)
      .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Digest mismatch: {e:?}")))?;

    let manifest_path = path::get(&self.name, &digest, path::FileType::Manifest)?;
    let tag_path = match &self.reference {
      Reference::Tag(tag) => Some(path::get(&self.name, tag, path::FileType::Tag)?),
      Reference::Sha256(_) => None,
    };

    match file::try_create(&manifest_path) {
      Ok(mut file) => {
        file.write_all(&data)?;
        if let Some(content_type) = content_type {
          set_xattr_media_type(&file, &content_type)?;
        }
      }
      Err(e) => match e.kind() {
        // If the file already exists, continue and create the tag
        io::ErrorKind::AlreadyExists => {
          println!("Info: Manifest already exists: {e:?}");
        }
        // Otherwise error out.
        _ => return Err(e),
      },
    };

    if let Some(tag_file_path) = tag_path {
      symlink::create_relative_symlink(&tag_file_path, &manifest_path)?;
    }

    Ok(())
  }

  /// Verifies that the manifest data matches its digest.
  fn verified_digest(&self, data: &[u8]) -> Result<String, DigestMismatch> {
    let computed_digest = digestor::get_sha256_digest(&data.to_vec());
    match &self.reference {
      Reference::Sha256(expected_digest) => {
        if computed_digest != *expected_digest {
          return Err(DigestMismatch {
            expected: expected_digest.clone(),
            computed: computed_digest,
          });
        }
        Ok(computed_digest)
      }
      Reference::Tag(_) => Ok(computed_digest),
    }
  }
}
