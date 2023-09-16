use ariane::compilation::compile::{self, CompileType};
use ariane::info_gathering::compiler::RustcInformation;
use ariane::sig::sig_generation::{hash_functions, FuzzyFunc};
use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use fuzzyhash::FuzzyHash;
use goblin::pe::section_table::SectionTable;
use log::{debug, error, info, log_enabled, Level};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::result;
use tar::Archive;

use ariane::functions_utils::search::{
    get_functions_from_bin, get_functions_from_lib, get_functions_from_pdb, FunctionType,
};
use ariane::functions_utils::search::{rva_to_pa, Function};
use ariane::info_gathering::krate::{self, Dependencies, Krate};
use ariane::sig::comparaison::compare;
use ariane::sig::comparaison::Symbol;

use crate::RecoverArgs;

#[derive(Serialize, Deserialize, Default)]
struct RecoveredSymbols {
    // dll_name: String,
    symbols: Vec<Symbol>,
}

#[derive(Serialize, Deserialize)]
/// {
///   "functions": [
///     {
///       "name": "sub_140001000",
///       "start": 5368713216,
///       "end": 5368713350
///     },
///     [... more entries ...]
///   ]
/// }
struct InutFunction {
    name: String,
    start: u32,
    end: u32,
}

#[derive(Serialize, Deserialize, Default)]
struct InputFunctions {
    functions: Vec<InutFunction>,
}

impl InputFunctions {
    pub fn to_functions<'data>(&self, file_content: &'data [u8]) -> Vec<Function<'data>> {
        let mut result = vec![];
        let parsed_pe = goblin::pe::PE::parse(file_content).unwrap();
        let mut section_map = HashMap::<u16, SectionTable>::new();
        for (i, section) in parsed_pe.sections.iter().enumerate() {
            section_map.insert(i as u16 + 1, section.clone());
        }

        for func in &self.functions {
            if let Some(start_pa) = rva_to_pa(&parsed_pe, func.start) {
                if let Some(end_pa) = rva_to_pa(&parsed_pe, func.end) {
                    result.push(Function {
                        rva: func.start,
                        data: &file_content[start_pa as usize..end_pa as usize],
                        name: Some(func.name.to_owned()),
                        fn_type: FunctionType::Exe,
                    })
                }
            }
        }

        result
    }
}

fn parse_input_fn_to_functions(filepath: &Path) -> Result<InputFunctions, Box<dyn Error>> {
    let file = std::fs::File::open(filepath)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u: InputFunctions = serde_json::from_reader(reader)?;

    Ok(u)
}

pub fn install_toolchain(compiler_version: &str) -> ExitStatus {
    let cmd = Command::new("rustup")
        .args(["install", compiler_version])
        .output()
        .expect("please install rustup");
    debug!(
        "{:?}, {:?}",
        String::from_utf8(cmd.stdout),
        String::from_utf8(cmd.stderr)
    );
    cmd.status
}

pub fn extract_targz(targz_path: &Path, dest_dir: &Path) -> Result<(), std::io::Error> {
    let mut archive = Archive::new(GzDecoder::new(std::fs::File::open(&targz_path).unwrap()));
    archive.unpack(&dest_dir)?;

    Ok(())
}

fn download_extract_compile(
    krate: &mut Krate,
    dest_dir: &Path,
    compiler_version: &str,
    compile_type: CompileType,
) -> Option<PathBuf> {
    let krate_full_name = format!("{}-{:#}", krate.name.clone(), krate.version);

    let extracted_path = dest_dir.join(PathBuf::from(&krate_full_name));
    let targz_path = match krate.download(&dest_dir) {
        Ok(path) => path,
        Err(_) => return None,
    };
    extract_targz(targz_path.as_path(), &dest_dir.join(dest_dir));
    compile::compile(
        &extracted_path.join("Cargo.toml"),
        &compiler_version,
        krate.get_features().unwrap(),
        compile_type,
    );

    let result_path = match compile_type {
        CompileType::Dylib => extracted_path.join(format!("target\\release\\{}.pdb", krate.name)),
        CompileType::StaticLib => extracted_path.join(format!(
            "target\\release\\lib{}.rlib",
            krate.name.replace("-", "_")
        )),
    };

    match result_path.exists() {
        true => Some(result_path),
        false => None,
    }
}

fn compile_hello_world_crate(compiler_version: &str) -> Option<PathBuf> {
    let cmd = Command::new("cargo")
        .args(["new", "hello_world_for_std", "--lib"])
        .current_dir(std::env::temp_dir().join("ariane"))
        .output()
        .expect("Could not find cargo.exe");
    debug!(
        "Exit status : {}\n{}",
        cmd.status,
        // String::from_utf8_lossy(cmd.stdout.as_ref()),
        String::from_utf8_lossy(cmd.stderr.as_ref())
    );
    let toml_path = std::env::temp_dir()
        .join("ariane\\hello_world_for_std")
        .join("Cargo.toml");

    compile::compile(&toml_path, compiler_version, &vec![], CompileType::Dylib);
    let result_path = std::env::temp_dir()
        .join("ariane\\hello_world_for_std\\target\\release\\hello_world_for_std.pdb");

    match result_path.exists() {
        true => Some(result_path),
        false => None,
    }
}

pub fn recover_subcommand(args: &RecoverArgs) -> Result<(), std::io::Error> {
    // let args = Arguments::parse();
    let bytes = std::fs::read(&args.target)?;
    let mut target_functions = vec![];

    if let Some(input_fn_file) = args.input_functions_file.clone() {
        let input_functions = parse_input_fn_to_functions(input_fn_file.as_path())
            .expect("Invalid file or malformed content");
        target_functions = input_functions.to_functions(&bytes);
    } else {
        target_functions =
            get_functions_from_bin(&bytes, 20).expect("Could not read functions from target");
    }

    info!("Target has {} functions", target_functions.len());

    let mut compiler_version = String::new();

    if let Some(compiler_info) = RustcInformation::from_buffer(&bytes) {
        let rustc_commit_hash = compiler_info.get_commit_hash();
        info!("{}", rustc_commit_hash.commit_hash_to_string());
        compiler_version = rustc_commit_hash
            .search_rustc_version()
            .expect("Could not find rustc version from your target !");
    }

    info!("Installing toolchain : {}", compiler_version);
    if !install_toolchain(compiler_version.as_str()).success() {
        panic!("Could not install toolchain {} !", compiler_version);
    }

    info!("Finding deps");
    let mut deps: Dependencies = Dependencies::from_buffer(&bytes);

    info!("Preparing donwload directory under %TEMP%\\ariane");
    let tmp_path = std::env::temp_dir();
    let projet_directory = tmp_path.clone().join("ariane");
    std::fs::create_dir_all(&projet_directory)?;

    // let mut compiled_dll_paths = vec![];
    let mut deps_krates = deps.get_dependencies_mut();

    let mut lib_functions: Vec<FuzzyFunc> = vec![];

    for cr in deps_krates {
        if let Some(lib_path) = download_extract_compile(
            cr,
            &projet_directory,
            &compiler_version,
            CompileType::StaticLib,
        ) {
            info!("Compiled {:?}", &lib_path);
            let lib_bytes = std::fs::read(lib_path.clone())
                .expect(&format!("Lib {:?} could not be read", &lib_path));
            let lib_fn = get_functions_from_lib(&lib_bytes);
            info!("{} functions found", lib_fn.len());
            lib_functions.append(&mut hash_functions(&lib_fn));
        } else {
            error!("Could not compile {:#}", cr);
        }
    }

    let std_crate_pdb =
        compile_hello_world_crate(&compiler_version).expect("Could not compile std crate");
    info!("Compiled {:?}", &std_crate_pdb);
    let mut dll_path = std_crate_pdb.clone();
    dll_path.set_extension("dll");
    let dll_bytes = std::fs::read(dll_path)?;
    let lib_fn = &get_functions_from_pdb(&dll_bytes, &std_crate_pdb)
        .expect("Could not extract functions from pdb");
    info!("{} functions found", lib_fn.len());

    lib_functions.append(&mut hash_functions(&lib_fn));

    info!("Hash target functions");
    let hashed_functions_target = hash_functions(&target_functions);

    let mut result: Vec<RecoveredSymbols> = vec![];
    let mut hash_dll_functions = HashMap::<String, String>::new();

    for hash in &lib_functions {
        if let Some(fn_name) = hash.name.as_ref() {
            if !hash_dll_functions.contains_key(fn_name) {
                hash_dll_functions.insert(fn_name.to_string(), hash.hash.hash.to_string());
            }
        }
    }

    info!("Compare target hashes with lib hashes");
    let syms = compare(&hashed_functions_target, &hash_dll_functions);

    result.push(RecoveredSymbols { symbols: syms });

    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(args.result_file.clone())
        .expect("File could not be open for writing");
    let j: String = serde_json::to_string(&result).unwrap();
    f.write(j.as_bytes())?;

    Ok(())
}
