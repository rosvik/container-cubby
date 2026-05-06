use std::collections::HashMap;

/// Annotations MUST be a key-value map where both the key and value MUST be
/// strings.
///
/// <https://github.com/opencontainers/image-spec/blob/v1.0.1/annotations.md#rules>
pub type Annotations = HashMap<String, String>;
