fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Vendored libgit2 uses Windows registry and security APIs from Advapi32.
        println!("cargo:rustc-link-lib=advapi32");
    }
}
