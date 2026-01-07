fn main() {
    let is_nightly = std::env::var("RUSTC_BOOTSTRAP").is_ok()
        || rustc_version::version_meta()
            .map(|m| m.channel == rustc_version::Channel::Nightly)
            .unwrap_or(false);

    if is_nightly {
        println!("cargo:rustc-cfg=is_nightly");
    }
}
