fn main() {
    println!("cargo:rerun-if-changed=src/build.rs");
    println!(
        "cargo:rustc-env=GIT_HASH={}",
        rustc_tools_util::get_commit_hash().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=COMMIT_DATE={}",
        rustc_tools_util::get_commit_date().unwrap_or_default()
    );
}
