use pdb::FallibleIterator;
use std::io::Write;
use std::{
    collections::HashMap,
    fs::OpenOptions,
    path::Path,
    process::Command,
};

pub fn compile(toml_path: &Path, toolchain_version: &str) {
    let _filtered_env: HashMap<String, String> = std::env::vars().into_iter().collect();

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(toml_path)
        .unwrap();

    // todo : check that [lib] and [profile.realese] are not duplicate entries
    file.write_all(
        String::from("\n[lib]\ncrate-type=[\"dylib\"]\n\n[profile.release]\ndebug=2\nsplit-debuginfo=\"packed\"\nstrip=\"none\"")
            .as_bytes(),
    );

    let toolchain = String::from(format!("+{}", toolchain_version));
    let mut args = vec![];
    args.push(toolchain.as_str());
    args.push("build");
    args.push("--release");
    args.push("--lib");

    let cmd = Command::new(String::from("cargo.exe"))
        .args(args)
        .current_dir(toml_path.parent().unwrap())
        .output()
        .expect("failed to execute process");
    println!(
        "{}",
        // String::from_utf8_lossy(cmd.stdout.as_ref()),
        String::from_utf8_lossy(cmd.stderr.as_ref())
    );
}
