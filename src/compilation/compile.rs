use std::io::Write;
use std::{collections::HashMap, fs::OpenOptions, path::Path, process::Command};
use toml_edit::{Array, Document, Formatted, Item, Value};

use crate::utils::toml_utils::add_array;

fn prepare_toml(node: &mut Document) {
    let mut elem = Array::new();
    elem.push("dylib");

    add_array(node, "lib", "crate-type", &Item::Value(Value::Array(elem)));
    // add_array( node, "profile.release", "debug", &Item::Value(Value::Integer(Formatted::new(2))));
    // add_array( node, "profile.release", "split-debuginfo", &Item::Value(Value::String(Formatted::new("packed".to_owned()))));
    // add_array( node, "profile.release", "strip", &Item::Value(Value::Boolean(Formatted::new(false))));
}

pub fn compile(toml_path: &Path, toolchain_version: &str, features: Vec<String>) {
    println!("Detected features: {:?}", features);
    let _filtered_env: HashMap<String, String> = std::env::vars().into_iter().collect();

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        // .append(true)
        .open(toml_path)
        .unwrap();
    file.flush().unwrap();

    let toml_content =
        String::from_utf8(std::fs::read(toml_path).expect("Could not read Cargo.toml content"))
            .unwrap();
    let mut document = toml_content.parse::<Document>().expect("Invalid Toml");
    prepare_toml(&mut document);
    file.write(document.to_string().as_bytes())
        .expect("Could not overwrite Cargo.toml file");
    file.flush().unwrap();

    let toolchain = String::from(format!("+{}", toolchain_version));
    let mut args = vec![];
    args.push(toolchain.as_str());
    args.push("build");
    // args.push("--all-features");
    args.push("--config");
    args.push("strip=false");
    args.push("--config");
    args.push("debug=2");
    args.push("--release");
    args.push("--lib");
    let mut features_string = String::new();

    if ! features.is_empty() {
        // args.push("--features");
        args.push("--features");
        for feature in &features {
            features_string.push_str(format!("{feature},").as_str());
        }

        // features_string.push_str(r#"""#);
        args.push(features_string.as_str());
    }
    
    println!("{:?}", args);
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
