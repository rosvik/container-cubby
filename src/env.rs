use std::env;

pub fn print_env_info() {
  let username = env::var("USERNAME");
  let password = env::var("PASSWORD");
  let auth_enabled = env::var("AUTH_ENABLED");

  if auth_enabled.is_ok_and(|x| x == "false") {
    println!("\x1b[1;33mWARNING: With AUTH_ENABLED=false, adding and deleting data from this registry can be done without authentication.\x1b[0m");
    println!("\x1b[1;33m         Not recommended for production.\x1b[0m");
  } else if username.is_err() || password.is_err() {
    println!(
      "\x1b[1;33mINFO: Username/password was not provided. Registry is in read-only mode.\x1b[0m"
    );
  };
}
