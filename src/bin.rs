use clap::Parser;
use goblin::pe::section_table::SectionTable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
mod compilation;
mod functions_utils;
mod info_gathering;
mod sig;
mod utils;

use ariane::functions_utils::search::{rva_to_pa, Function};
use ariane::functions_utils::search::get_functions;
use ariane::sig::comparaison::compare;
use ariane::info_gathering::compiler::{finc_compiler_version, find_tag_from_hash};
use ariane::info_gathering::krate::{Krate, find_deps};
use ariane::sig::comparaison::Symbol;
use ariane::{install_toolchain, handle};

#[derive(Serialize, Deserialize, Default)]
struct RecoveredSymbols {
    dll_name: String,
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
    pub fn to_functions(&self, file_content: &[u8]) -> Vec<Function> {
        let mut result = vec![];
        let parsed_pe = goblin::pe::PE::parse(file_content).unwrap();
        let mut section_map = HashMap::<u16, SectionTable>::new();
        for (i, section) in parsed_pe.sections.iter().enumerate() {
            section_map.insert(i as u16 + 1, section.clone());
        }

        for func in &self.functions {
            result.push(Function {
                start_rva: func.start,
                end_rva: func.end,
                start_pa: rva_to_pa(&parsed_pe, func.start).unwrap(),
                end_pa: rva_to_pa(&parsed_pe, func.end).unwrap(),
                name: Some(func.name.to_owned()),
            })
        }

        result
    }
}

#[derive(Parser, Debug)]
#[clap(version)]
struct Arguments {
    #[clap(required = true)]
    file: String,
    /// Path of a file containing a list of your target binary functions. See README.md.
    #[clap(short, long, required = false)]
    input_functions_file: Option<PathBuf>,
    #[clap(required = true)]
    output: String,
}

fn parse_input_fn_to_functions(filepath: &Path) -> Result<InputFunctions, Box<dyn Error>> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u: InputFunctions = serde_json::from_reader(reader)?;

    Ok(u)
}

fn main() -> Result<(), std::io::Error> {
    let args = Arguments::parse();
    let mut functions = vec![];
    let bytes = match std::fs::read(&args.file) {
        Ok(b) => b,
        Err(e) => {
            writeln!(std::io::stderr(), "Could not read file : {}", e)?;
            return Err(e);
        }
    };

    if let Some(input_fn_file) = args.input_functions_file {
        let input_functions = parse_input_fn_to_functions(input_fn_file.as_path())
            .expect("Invalid file or malformed content");
        functions = input_functions.to_functions(bytes.as_ref());
    } else {
        let file_path: PathBuf = PathBuf::from(args.file);
        functions = get_functions(file_path.as_path(), None);
    }

    let compiler_commit = match finc_compiler_version(&bytes) {
        Some(version) => version,
        None => {
            println!("No rustc compiler version found in your target");
            return Ok(());
        }
    };

    println!("Compiler commit {:#}", compiler_commit);

    let compiler_tag = match find_tag_from_hash(&compiler_commit.as_str()) {
        Some(tag) => {
            println!("Compiler tag {:#}. This version will be used for compilation. If results aren't accurate enough, please download and compile the exact compiler version using the commit hash given above.", tag);

            tag.to_string()
        }
        // git ls-remote https://github.com/rust-lang/rust | sort | grep tag | grep '/1.7'
        None => "1.72.0".to_string(),
    };
    install_toolchain(compiler_tag.as_str());

    println!("Finding deps");
    let mut compiled_dll_paths = vec![];
    let found_crates: Vec<Krate> = find_deps(&bytes);
    for cr in found_crates {
        println!("{:#}", cr);
        if let Some(path) = handle(cr, compiler_tag.as_str()) {
            compiled_dll_paths.push(path);
        };
    }
    let mut result: Vec<RecoveredSymbols> = vec![];
    for dll in &compiled_dll_paths {
        println!("Analysing {}", dll);
        let file_path: PathBuf = PathBuf::from(dll);
        let mut pdb_path: PathBuf = PathBuf::from(dll);
        pdb_path.set_extension("pdb");
        let dll_bytes = std::fs::read(file_path).unwrap();
        let mut dll_functions = get_functions(Path::new(dll), Some(pdb_path.as_path()));
        let syms = compare(
            bytes.as_ref(),
            &functions,
            dll_bytes.as_ref(),
            &dll_functions,
        );
        result.push(RecoveredSymbols {
            dll_name: dll.clone(),
            symbols: syms,
        });
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(args.output)
        .expect("File could not be open for writing");
    let j: String = serde_json::to_string(&result).unwrap();
    f.write(j.as_bytes());

    Ok(())
}
