use super::*;
use std::iter::repeat_with;

pub fn get_random_namespace() -> String {
  let random_string: String = repeat_with(fastrand::lowercase).take(10).collect();
  format!("test-{CRATE_VERSION}-{}", random_string)
}
