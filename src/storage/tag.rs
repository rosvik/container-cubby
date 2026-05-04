use crate::{
  digest::Digest,
  storage::{path, symlink},
  utils::is_safe_tag,
};

/// A custom, human-readable pointer to a manifest. A manifest digest may have
/// zero, one, or many tags referencing it.
pub struct Tag {
  pub name: String,
  pub reference: String,
  pub manifest_digest: Digest,
}

impl Tag {
  pub fn new(
    name: &str,
    reference: String,
    manifest_digest: Digest,
  ) -> Result<Self, std::io::Error> {
    if !is_safe_tag(&reference) {
      return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid reference"));
    }

    Ok(Self {
      name: name.to_string(),
      reference,
      manifest_digest,
    })
  }

  pub fn create(&self) -> Result<(), std::io::Error> {
    let tag_path = path::get(&self.name, &self.reference.to_string(), path::FileType::Tag)?;
    let manifest_path =
      path::get(&self.name, &self.manifest_digest.to_string(), path::FileType::Manifest)?;

    symlink::create_relative_symlink(&tag_path, &manifest_path)
  }
}
