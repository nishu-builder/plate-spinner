use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
    println!("cargo:rerun-if-changed=build.rs");
}
