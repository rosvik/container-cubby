use crate::utils::is_safe_digest;
use serde::Deserialize;
use sha2::Digest as _;
use std::fmt::Display;

#[derive(Clone, Copy)]
pub enum Algorithm {
  Sha256,
}
impl Display for Algorithm {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Algorithm::Sha256 => write!(f, "sha256"),
    }
  }
}
impl PartialEq for Algorithm {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Algorithm::Sha256, Algorithm::Sha256) => true,
    }
  }
}
pub struct Digest {
  pub algorithm: Algorithm,
  pub hex: String,
}

impl Digest {
  pub fn new(algorithm: Algorithm, data: &Vec<u8>) -> Self {
    let hash = match algorithm {
      Algorithm::Sha256 => sha256(data),
    };
    let hex = bytes_to_hex(&hash);
    Self { algorithm, hex }
  }

  pub fn from_string(digest: &str) -> Result<Self, Box<dyn std::error::Error>> {
    let (algorithm, hex) = digest.split_once(':').ok_or("Invalid digest")?;
    let algorithm = match algorithm {
      "sha256" => Algorithm::Sha256,
      _ => return Err("Invalid algorithm".into()),
    };
    if !is_safe_digest(format!("{algorithm}:{hex}").as_str()) {
      return Err("Invalid digest".into());
    }
    Ok(Self {
      algorithm,
      hex: hex.to_string(),
    })
  }

  pub fn prefix(&self) -> String {
    self.hex.chars().take(2).collect::<String>()
  }
}
impl Display for Digest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}", self.algorithm, self.hex)
  }
}
impl PartialEq for Digest {
  fn eq(&self, other: &Self) -> bool {
    self.algorithm == other.algorithm && self.hex == other.hex
  }
}
impl From<String> for Digest {
  fn from(digest: String) -> Self {
    Self::from_string(digest.as_str()).unwrap()
  }
}
impl<'de> Deserialize<'de> for Digest {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::de::Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    Ok(Self::from_string(s.as_str()).unwrap())
  }
}

fn sha256(data: &Vec<u8>) -> Vec<u8> {
  let mut hasher: sha2::Sha256 = sha2::Sha256::new();
  sha2::Digest::update(&mut hasher, data);
  hasher.finalize().to_vec()
}

fn bytes_to_hex(data: &Vec<u8>) -> String {
  let mut s = String::new();
  for byte in data {
    s.push_str(&format!("{byte:02x}"));
  }
  s
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::digest::Algorithm::Sha256;

  fn hex_string_to_bytes(hex_string: &str) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();
    let mut chars = hex_string.chars();
    while let Some(a) = chars.next() {
      let b = chars.next().unwrap();
      let byte = u8::from_str_radix(&format!("{a}{b}"), 16).unwrap();
      data.push(byte);
    }
    data
  }

  const EMPTY_STRING: &str = "";
  const EMPTY_HEX_STRING: &str = "";
  const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
  const EXAMPLE_STRING: &str = "hello world";
  const EXAMPLE_HEX_STRING: &str = "68656c6c6f20776f726c64";
  const EXAMPLE_HASH: &str = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

  #[test]
  fn test_bytes_to_hex_string() {
    let empty_bytes: Vec<u8> = EMPTY_STRING.as_bytes().to_vec();
    assert_eq!(EMPTY_HEX_STRING, bytes_to_hex(&empty_bytes));
    let example_bytes: Vec<u8> = EXAMPLE_STRING.as_bytes().to_vec();
    assert_eq!(EXAMPLE_HEX_STRING, bytes_to_hex(&example_bytes));
  }

  #[test]
  fn test_hex_string_to_bytes() {
    let empty_bytes: Vec<u8> = EMPTY_STRING.as_bytes().to_vec();
    assert_eq!(empty_bytes, hex_string_to_bytes(EMPTY_HEX_STRING));
    let example_bytes: Vec<u8> = EXAMPLE_STRING.as_bytes().to_vec();
    assert_eq!(example_bytes, hex_string_to_bytes(EXAMPLE_HEX_STRING));
  }

  #[test]
  fn test_hash_data() {
    let empty_bytes: Vec<u8> = EMPTY_STRING.as_bytes().to_vec();
    assert_eq!(hex_string_to_bytes(EMPTY_HASH), sha256(&empty_bytes));
    let example_bytes: Vec<u8> = EXAMPLE_STRING.as_bytes().to_vec();
    assert_eq!(hex_string_to_bytes(EXAMPLE_HASH), sha256(&example_bytes));
  }

  #[test]
  fn test_get_sha256_digest() {
    let empty_digest = format!("sha256:{EMPTY_HASH}");
    let empty_bytes: Vec<u8> = EMPTY_STRING.as_bytes().to_vec();
    assert_eq!(empty_digest, Digest::new(Sha256, &empty_bytes).to_string());
    let example_digest = format!("sha256:{EXAMPLE_HASH}");
    let example_bytes: Vec<u8> = EXAMPLE_STRING.as_bytes().to_vec();
    assert_eq!(example_digest, Digest::new(Sha256, &example_bytes).to_string());
  }

  #[test]
  fn test_mainifest_fixture_digest() {
    let manifest = include_str!("./tests/fixtures/image_manifest.json").as_bytes().to_vec();
    let manifest_sha = "sha256:edee272db7445c0aedfa7892df3f734fa6117221e37389063e65648ba47f7b00";
    let digest = Digest::new(Sha256, &manifest);
    assert_eq!(manifest_sha, digest.to_string());
  }
}
