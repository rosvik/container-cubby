use crate::{
  digest::{Algorithm::Sha256, Digest},
  storage::{ensure_container_dir_exists, file, path, tag::Tag, xattr::set_xattr_media_type},
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

    let reference = verify_reference(reference)
      .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid reference"))?;

    Ok(Self {
      name: name.to_string(),
      reference,
    })
  }

  /// Creates the manifest file in the container directory, verifies the digest,
  /// and creates a tag symlink if the reference is a tag.
  ///
  /// Returns the verified digest of the manifest.
  pub fn create_manifest(
    &self,
    data: Vec<u8>,
    content_type: Option<String>,
  ) -> Result<Digest, io::Error> {
    ensure_container_dir_exists(&self.name)?;

    // Verify that the digest matches the data
    let digest = self
      .verified_digest(&data)
      .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Digest mismatch: {e:?}")))?;

    let manifest_path = path::get(&self.name, &digest.to_string(), path::FileType::Manifest)?;

    // Initialize tag early in case of errors
    let tag = match self.reference {
      Reference::Tag(ref reference) => {
        Some(Tag::new(&self.name, reference.clone(), digest.clone())?)
      }
      _ => None,
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

    if let Some(tag) = tag {
      tag.create()?;
    }

    Ok(digest)
  }

  /// If the reference is a digest, verifies that the digest matches the data,
  /// otherwise returns the computed digest.
  fn verified_digest(&self, data: &[u8]) -> Result<Digest, DigestMismatch> {
    match &self.reference {
      Reference::Digest(expected) => {
        let computed = Digest::new(expected.algorithm, &data.to_vec());
        match computed == *expected {
          true => Ok(computed),
          false => Err(DigestMismatch {
            expected: expected.clone(),
            computed,
          }),
        }
      }
      Reference::Tag(_) => Ok(Digest::new(Sha256, &data.to_vec())),
    }
  }
}
