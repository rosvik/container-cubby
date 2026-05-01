use sha2::Digest as _;

// https://github.com/opencontainers/image-spec/blob/v1.0.1/descriptor.md#digests
//
// Where the client provided digest is C, and client provided data is D:
// C == get_sha256_digest(D) == 'sha256:' + bytes_to_hex_string(hash_data(D))

pub fn get_sha256_digest(data: &Vec<u8>) -> String {
  let hash_bytes = hash_data(data);
  let hash_string = bytes_to_hex_string(&hash_bytes);
  format!("sha256:{hash_string}")
}

fn hash_data(data: &Vec<u8>) -> Vec<u8> {
  let mut hasher: sha2::Sha256 = sha2::Sha256::new();
  sha2::Digest::update(&mut hasher, data);
  hasher.finalize().to_vec()
}

fn bytes_to_hex_string(data: &Vec<u8>) -> String {
  let mut s = String::new();
  for byte in data {
    s.push_str(&format!("{byte:02x}"));
  }
  s
}

#[cfg(test)]
mod tests {
  use super::*;

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
    assert_eq!(EMPTY_HEX_STRING, bytes_to_hex_string(&empty_bytes));
    let example_bytes: Vec<u8> = EXAMPLE_STRING.as_bytes().to_vec();
    assert_eq!(EXAMPLE_HEX_STRING, bytes_to_hex_string(&example_bytes));
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
    assert_eq!(hex_string_to_bytes(EMPTY_HASH), hash_data(&empty_bytes));
    let example_bytes: Vec<u8> = EXAMPLE_STRING.as_bytes().to_vec();
    assert_eq!(hex_string_to_bytes(EXAMPLE_HASH), hash_data(&example_bytes));
  }

  #[test]
  fn test_get_sha256_digest() {
    let empty_digest = format!("sha256:{EMPTY_HASH}");
    let empty_bytes: Vec<u8> = EMPTY_STRING.as_bytes().to_vec();
    assert_eq!(empty_digest, get_sha256_digest(&empty_bytes));
    let example_digest = format!("sha256:{EXAMPLE_HASH}");
    let example_bytes: Vec<u8> = EXAMPLE_STRING.as_bytes().to_vec();
    assert_eq!(example_digest, get_sha256_digest(&example_bytes));
  }

  #[test]
  fn test_mainifest_fixture_digest() {
    let manifest = include_str!("./tests/fixtures/image_manifest.json").as_bytes().to_vec();
    let manifest_sha = "sha256:edee272db7445c0aedfa7892df3f734fa6117221e37389063e65648ba47f7b00";
    assert_eq!(manifest_sha, get_sha256_digest(&manifest));
  }
}
