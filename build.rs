use copy_to_output::copy_to_output;
use std::env;

#[allow(unused_must_use)]
fn main() {
    println!("cargo:rerun-if-changed=assets/*");
    copy_to_output("assets", &env::var("PROFILE").unwrap());
}
