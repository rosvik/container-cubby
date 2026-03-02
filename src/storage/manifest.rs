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

  /// Creates a manifest file and optionally a tag symlink, and returns the
  /// manifest file with write access. If the tag exists, it is overwritten.
  pub fn create_manifest(
    &self,
    data: Vec<u8>,
    content_type: Option<String>,
  ) -> Result<(), io::Error> {
    ensure_container_dir_exists(&self.name)?;

    let digest = match &self.reference {
      Reference::Sha256(digest) => {
        // Verify that the digest matches the data
        if let Err(e) = self.verify(&data) {
          return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Digest mismatch: {e:?}"),
          ));
        }
        digest.clone()
      }
      Reference::Tag(_) => digestor::get_sha256_digest(&data.to_vec()),
    };

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
  fn verify(&self, data: &[u8]) -> Result<(), DigestMismatch> {
    match &self.reference {
      Reference::Sha256(digest) => {
        let computed_digest = digestor::get_sha256_digest(&data.to_vec());
        if computed_digest != *digest {
          return Err(DigestMismatch {
            expected: digest.to_string(),
            computed: computed_digest,
          });
        }
        Ok(())
      }
      Reference::Tag(_) => Ok(()),
    }
  }
}
