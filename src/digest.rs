use sha2::{Digest, Sha256};

/*
https://github.com/opencontainers/image-spec/blob/v1.0.1/descriptor.md#digests

The digest property of a Descriptor acts as a content identifier, enabling
content addressability. It uniquely identifies content by taking a collision-
resistant hash of the bytes. If the digest can be communicated in a secure
manner, one can verify content from an insecure source by recalculating the
digest independently, ensuring the content has not been modified.

Example:
sha256:6c3c624b58dbbcd3c0dd82b4c53f04194d1247c6eebdaab7c610cf7d66709b3b

A digest is calculated by the following pseudo-code, where H is the selected
hash algorithm, identified by string <alg>:

```
let ID(C) = Descriptor.digest
let C = <bytes>
let D = '<alg>:' + Encode(H(C))
let verified = ID(C) == D
```

Above, we define the content identifier as ID(C), extracted from the
Descriptor.digest field. Content C is a string of bytes. Function H returns the
hash of C in bytes and is passed to function Encode and prefixed with the
algorithm to obtain the digest. The result verified is true if ID(C) is equal to
D, confirming that C is the content identified by D.

After verification, the following is true:
D == ID(C) == '<alg>:' + Encode(H(C))
*/

pub fn get_sha256_digest(data: &Vec<u8>) -> String {
  let hash_bytes = hash_data(data);
  let hash_string = bytes_to_hex_string(&hash_bytes);
  format!("sha256:{}", hash_string)
}

fn hash_data(data: &Vec<u8>) -> Vec<u8> {
  let mut hasher: sha2::Sha256 = Sha256::new();
  sha2::Digest::update(&mut hasher, data);
  hasher.finalize().to_vec()
}

fn bytes_to_hex_string(data: &Vec<u8>) -> String {
  let mut s = String::new();
  for byte in data {
    s.push_str(&format!("{:02x}", byte));
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
      let byte = u8::from_str_radix(&format!("{}{}", a, b), 16).unwrap();
      data.push(byte);
    }
    data
  }

  #[test]
  fn test_bytes_to_hex_string() {
    // Empty string
    let empty_string = "";
    let empty_bytes: Vec<u8> = empty_string.as_bytes().to_vec();
    let empty_hex_string = "";
    assert_eq!(empty_hex_string, bytes_to_hex_string(&empty_bytes));

    // Example string
    let example_string = "hello world";
    let example_bytes: Vec<u8> = example_string.as_bytes().to_vec();
    let example_hex_string = "68656c6c6f20776f726c64";
    assert_eq!(example_hex_string, bytes_to_hex_string(&example_bytes));
  }

  #[test]
  fn test_hex_string_to_bytes() {
    // Empty string
    let empty_string = "";
    let empty_bytes: Vec<u8> = empty_string.as_bytes().to_vec();
    let empty_hex_string = "";
    assert_eq!(empty_bytes, hex_string_to_bytes(empty_hex_string));

    // Example string
    let example_string: &str = "hello world";
    let example_bytes: Vec<u8> = example_string.as_bytes().to_vec();
    let example_hex_string = "68656c6c6f20776f726c64";
    assert_eq!(example_bytes, hex_string_to_bytes(example_hex_string));
  }

  #[test]
  fn test_hash_data() {
    // Empty string
    let empty_string = "";
    let empty_bytes: Vec<u8> = empty_string.as_bytes().to_vec();
    let empty_hash: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    assert_eq!(hex_string_to_bytes(empty_hash), hash_data(&empty_bytes));

    // Example string
    let example_string: &str = "hello world";
    let example_bytes: Vec<u8> = example_string.as_bytes().to_vec();
    let example_hash: &str = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    assert_eq!(hex_string_to_bytes(example_hash), hash_data(&example_bytes));
  }

  #[test]
  fn test_get_sha256_digest() {
    // Empty string
    let empty_string = "";
    let empty_bytes: Vec<u8> = empty_string.as_bytes().to_vec();
    let empty_digest: &str =
      "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    assert_eq!(empty_digest, get_sha256_digest(&empty_bytes));

    // Example string
    let example_string: &str = "hello world";
    let example_bytes: Vec<u8> = example_string.as_bytes().to_vec();
    let example_digest: &str =
      "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    assert_eq!(example_digest, get_sha256_digest(&example_bytes));
  }
}
