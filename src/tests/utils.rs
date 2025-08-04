use super::*;
use std::iter::repeat_with;

pub fn get_random_namespace() -> String {
  let random_string: String = repeat_with(fastrand::lowercase).take(10).collect();
  let namespace = format!("test-{}/{}", env::CRATE_VERSION, random_string);
  println!("Using namespace: {namespace}");
  namespace
}
