use crates_io_api::SyncClient;
use flate2::read::GzDecoder;

use info_gathering::krate::Krate;
use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use tar::Archive;

use crate::compilation::compile::compile;
pub mod compilation;
pub mod functions_utils;
pub mod info_gathering;
pub mod sig;
pub mod utils;

#[derive(Debug)]
pub enum ArianeError {
    InvalidInput,
    IOError(std::io::Error),
}

pub fn install_toolchain(compiler_version: &str) {
    let cmd = Command::new("rustup")
        .args(["install", compiler_version])
        .output()
        .expect("please install rustup");
    println!(
        "{:?}, {:?}",
        String::from_utf8(cmd.stdout),
        String::from_utf8(cmd.stderr)
    );
}

// This NEEDs refacto
pub fn handle(c: Krate, compiler_version: &str) -> Option<String> {
    // let compiler_version = "1.64.0";
    // DOWNLOAD
    let reqwest_client = reqwest::blocking::Client::new();
    let tmp_path = std::env::temp_dir();
    let projet_directory = tmp_path.clone().join("cerb");
    fs::create_dir_all(&projet_directory);
    
    let target_dir = PathBuf::from(projet_directory);
    
    let client = SyncClient::new(
        "Ariane (https://github.com/N0fix/Ariane)",
        std::time::Duration::from_millis(1_0000),
    )
    .unwrap();
    
    let metadata = client.get_crate(&c.name.as_str()).unwrap();
    // println!("meta {:?}", metadata);
    
    let v = metadata
        .versions
        .iter()
        .find(|v| v.num == c.version.to_string())
        .unwrap();
    // let features = vec![];
    println!("{:?}", v.features);
    
    let krates = c.clone();
    let set_a: HashSet<String> = v.features.keys().map(|x| x.clone()).collect();
    let set_b: HashSet<String> = krates.features.into_iter().collect();
    let features: Vec<String> = set_a.intersection(&set_b).map(|s| s.clone()).collect();


    let dl_url = format!("https://crates.io{}", v.dl_path);
    // println!("Download url : {:#}", dl_url);
    
    // WRITE TO DISK
    let response = reqwest_client.get(&dl_url).send().unwrap();
    let tarball_path = target_dir.clone().join(format!("{:#}.tar.gz", c));
    // println!("Tarball path : {:?}", tarball_path);
    let mut tarball_file = fs::File::create(&tarball_path).unwrap();

    let mut content = Cursor::new(response.bytes().unwrap());

    std::io::copy(&mut content, &mut tarball_file).unwrap();

    // EXTRACT
    let _tarball_file = fs::File::open(&tarball_path).unwrap();
    let mut archive = Archive::new(GzDecoder::new(fs::File::open(&tarball_path).unwrap()));
    archive.unpack(&target_dir).unwrap();
    {
        let cargo_toml_path = target_dir
            .clone()
            .join(format!("{:#}", c))
            .join("Cargo.toml");

        // PATCH NO_STD
        archive = Archive::new(GzDecoder::new(fs::File::open(&tarball_path).unwrap()));
        for file in archive.entries().unwrap() {
            let f = file.unwrap();
            if f.path().unwrap().file_name().unwrap() == "lib.rs" {
                let librs_path = target_dir.clone().join(f.path().unwrap());
                let librs_content =
                    std::fs::read_to_string(&librs_path).expect("Could not read lib.rs");
                let librs_content = librs_content.replace("#![no_std]", "");
                let librs_content =
                    librs_content.replace("#![cfg_attr(not(feature = \"std\"), no_std)]", "");

                std::fs::write(&librs_path, librs_content).expect("Could not write lib.rs file");
            }
        }

        // COMPILE
        compile(cargo_toml_path.as_path(), compiler_version, features);

        let result_path = cargo_toml_path
            .parent()
            .unwrap()
            .join("target")
            .join("release")
            .join(format!("{}.dll", c.name));

        let _result_pdb = cargo_toml_path
            .parent()
            .unwrap()
            .join("target")
            .join("release")
            .join(format!("{}.pdb", c.name));
        if let Ok(_) = fs::metadata(&result_path) {
            return Some(result_path.to_str().unwrap().to_string());
        }
    }

    None
}
