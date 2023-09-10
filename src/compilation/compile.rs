use log::{debug, error, info, log_enabled, Level};
use std::fmt;
use std::io::Write;
use std::process::ExitStatus;
use std::{collections::HashMap, fs::OpenOptions, path::Path, process::Command};
use toml_edit::{Array, Document, Formatted, Item, Value};

use crate::info_gathering::krate::Krate;
use crate::utils::toml_utils::add_array;

#[derive(Debug, Copy, Clone)]
pub enum CompileType {
    Dylib,
    StaticLib,
}

impl fmt::Display for CompileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            CompileType::Dylib => write!(f, "dylib"),
            CompileType::StaticLib => write!(f, "rlib"),
        }
    }
}

fn set_crate_type(node: &mut Document, compile_type: &CompileType) {
    let mut elem = Array::new();
    elem.push(compile_type.to_string());

    add_array(node, "lib", "crate-type", &Item::Value(Value::Array(elem)));
    // add_array( node, "profile.release", "debug", &Item::Value(Value::Integer(Formatted::new(2))));
    // add_array( node, "profile.release", "split-debuginfo", &Item::Value(Value::String(Formatted::new("packed".to_owned()))));
    // add_array( node, "profile.release", "strip", &Item::Value(Value::Boolean(Formatted::new(false))));
}

pub fn compile(
    toml_path: &Path,
    toolchain_version: &str,
    features: &Vec<String>,
    compile_type: CompileType,
) -> ExitStatus {
    debug!("Patching toml : {:?}", toml_path);
    let mut document = std::fs::read_to_string(toml_path)
        .unwrap()
        .parse::<Document>()
        .expect("Invalid Toml");
    set_crate_type(&mut document, &compile_type);

    {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(toml_path)
            .unwrap();

        file.write(document.to_string().as_bytes())
            .expect("Could not overwrite Cargo.toml file");
    }

    let toolchain = String::from(format!("+{}", toolchain_version));

    let mut args: Vec<&str> = vec![
        toolchain.as_str(),
        "build",
        "--config",
        "strip=false",
        "--config",
        "debug=2",
        "--release",
        "--lib",
    ];

    let mut features_string = String::new();

    if !features.is_empty() {
        args.push("--features");

        for feature in features {
            features_string.push_str(format!("{feature},").as_str());
        }

        args.push(features_string.as_str());
    }

    debug!("Compiling with args : {:?}", args);
    let cmd = Command::new(String::from("cargo.exe"))
        .args(args)
        .current_dir(toml_path.parent().unwrap())
        .output()
        .expect("failed to execute process");
    debug!(
        "Exit status : {}\n{}",
        cmd.status,
        // String::from_utf8_lossy(cmd.stdout.as_ref()),
        String::from_utf8_lossy(cmd.stderr.as_ref())
    );
    cmd.status
}
