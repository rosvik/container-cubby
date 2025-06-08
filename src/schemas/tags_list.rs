use serde::{Deserialize, Serialize};

/// https://github.com/opencontainers/distribution-spec/blob/main/spec.md#listing-tags
#[derive(Debug, Serialize, Deserialize)]
pub struct TagsList {
  /// <name> is the namespace of the repository.
  pub name: String,

  /// <tags> are each tags on the repository.
  pub tags: Vec<String>,
}
