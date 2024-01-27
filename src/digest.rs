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

pub fn hash_data(data: &Vec<u8>) -> Vec<u8> {
    let mut hasher: sha2::Sha256 = Sha256::new();
    sha2::Digest::update(&mut hasher, data);
    hasher.finalize().to_vec()
}

pub fn to_hex_string(data: Vec<u8>) -> String {
    let mut s = String::new();
    for byte in data {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}
