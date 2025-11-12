
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=scripts/copy-bundles.sh");
    println!("cargo:rerun-if-changed=../fetch-core/bundle/");

    println!("cargo:warning=Copying fetch-core/bundle contents to target directory...");
    let status = std::process::Command::new("sh")
        .arg("scripts/copy-bundles.sh")
        .status()
        .expect("Failed to execute copy-bundles.sh script");

    if !status.success() {
        println!("cargo:error=copy-bundles.sh script failed with status: {}", status);
    }
}