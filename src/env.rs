use crate::utils::ansi::{RESET, YELLOW};

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub fn crate_info() -> String {
  format!("{CRATE_NAME} v{CRATE_VERSION}")
}

pub const PROTOCOL: &str = "http";
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 8602;
pub fn host() -> String {
  std::env::var("HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string())
}
pub fn port() -> u16 {
  std::env::var("PORT").unwrap_or_default().parse::<u16>().unwrap_or(DEFAULT_PORT)
}

pub fn username() -> Option<String> {
  std::env::var("USERNAME").ok()
}
pub fn password() -> Option<String> {
  std::env::var("PASSWORD").ok()
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthMode {
  None,
  ReadWrite,
  WriteOnly,
}
pub fn auth_mode() -> AuthMode {
  match std::env::var("AUTH_MODE").unwrap().as_str() {
    "none" => AuthMode::None,
    "read_write" => AuthMode::ReadWrite,
    "write_only" => AuthMode::WriteOnly,
    invalid => panic!("Invalid AUTH_MODE: {invalid}"),
  }
}

pub const DEFAULT_DATA_DIR: &str = "data";
pub const DEFAULT_TEST_DATA_DIR: &str = "test-data";
pub fn data_dir() -> String {
  if cfg!(test) {
    return std::env::var("TEST_DATA_DIR").unwrap_or_else(|_| DEFAULT_TEST_DATA_DIR.to_string());
  }
  std::env::var("DATA_DIR").unwrap_or_else(|_| DEFAULT_DATA_DIR.to_string())
}

pub fn prune_cron() -> Option<String> {
  std::env::var("PRUNE_CRON").ok()
}

pub fn print_env_info() {
  let username = std::env::var("USERNAME");
  let password = std::env::var("PASSWORD");
  let auth_enabled = std::env::var("AUTH_ENABLED");

  if auth_enabled.is_ok_and(|x| x == "false") {
    println!(
      "{YELLOW}WARNING: With AUTH_ENABLED=false, adding and deleting data from this registry can be done without authentication.{RESET}"
    );
    println!("{YELLOW}         Not recommended for production.{RESET}");
  } else if username.is_err() || password.is_err() {
    println!(
      "{YELLOW}INFO: Username/password was not provided. Registry is in read-only mode.{RESET}"
    );
  };
}
