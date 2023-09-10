use log::{debug, error, info, log_enabled, Level};
use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;
use std::{fs, path::Path};

use fuzzyhash::FuzzyHash;
use goblin::pe::section_table::SectionTable;
use goblin::pe::Coff;
use goblin::{archive, pe};
use itertools::Itertools;
use object::coff;
use pdb::{FallibleIterator, ImageSectionHeader};

use crate::sig::sig_generation::{hash_single_func, FuzzyFunc, Hash};

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum FunctionType {
    Pdb,
    Lib,
    Exe,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct Function<'data> {
    pub data: &'data [u8],
    pub name: Option<String>,
    /// Meaning of this field depends on the `FunctionType`.
    /// It is the RVA of the function for `FunctionType::Exe`.
    /// Otherwise it is reserved field.
    pub rva: u32,
    pub fn_type: FunctionType,
}

impl<'data> Display for Function<'data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{:x?} : {:?}", self.rva, self.name)
    }
}

const VALID_MIN_BYTES_FUNCTION_TRESHOLD: u32 = 50;

pub fn rva_to_pa(pe: &goblin::pe::PE, rva: u32) -> Option<u32> {
    for section in &pe.sections {
        if rva >= section.virtual_address && rva < section.virtual_size + section.virtual_address {
            return Some(rva - section.virtual_address + section.pointer_to_raw_data);
        }
    }

    None
}

fn guess_function_size(function_bytes: &[u8]) -> u32 {
    let mut padding_bytes = 0;
    let mut function_size = 0;

    for b in function_bytes.iter() {
        if *b == 0xCC {
            padding_bytes += 1;
        } else {
            padding_bytes = 0;
        }

        if padding_bytes > 2 {
            break;
        }

        function_size += 1;
    }

    function_size - padding_bytes
}

fn get_exception_data_functions(file: &[u8]) -> Vec<Function> {
    let mut functions = vec![];
    let parsed_pe = goblin::pe::PE::parse(file).unwrap();
    if let Some(except_data) = &parsed_pe.exception_data {
        for f in except_data.functions() {
            let f = f.unwrap();
            if let Some(start_pa) = rva_to_pa(&parsed_pe, f.begin_address) {
                if let Some(end_pa) = rva_to_pa(&parsed_pe, f.end_address) {
                    if start_pa + end_pa < file.len() as u32 {
                        functions.push(Function {
                            rva: f.begin_address,
                            data: &file[start_pa as usize..end_pa as usize],
                            // todo : search for symbols in dwarf
                            name: None,
                            fn_type: FunctionType::Exe,
                        });
                    }
                }
            }
        }
    }

    functions
}

pub fn get_functions_from_pdb<'data>(
    exe_bytes: &'data [u8],
    pdb_path: &Path,
) -> Result<Vec<Function<'data>>, std::io::Error> {
    pdb_path.try_exists()?;

    let mut section_map = HashMap::<u16, ImageSectionHeader>::new();
    let file = std::fs::File::open(pdb_path)?;

    let mut pdb = pdb::PDB::open(file).unwrap();

    let sections = pdb.sections().unwrap().unwrap();
    for (i, section) in sections.iter().enumerate() {
        section_map.insert(i as u16 + 1, section.clone());
    }

    let symbol_table = pdb.global_symbols().unwrap();

    let mut symbols = symbol_table.iter();

    let mut map = HashMap::<u32, (u32, String)>::new();
    let mut result_functions = vec![];

    while let Some(symbol) = symbols.next().unwrap() {
        // println!("{:?}", symbol);
        match symbol.parse().unwrap() {
            pdb::SymbolData::Public(func) => {
                // println!("{:?}", func);
                if section_map.contains_key(&func.offset.section) {
                    let pa =
                        func.offset.offset + section_map[&func.offset.section].pointer_to_raw_data;
                    let va = func.offset.offset + section_map[&func.offset.section].virtual_address;
                    map.insert(
                        pa,
                        (
                            va,
                            String::from_utf8(func.name.as_bytes().to_vec()).unwrap(),
                        ),
                    );
                }
            }
            _ => {}
        }
    }

    // println!("Module private symbols:");
    let dbi = pdb.debug_information().unwrap();
    let mut modules = dbi.modules().unwrap();
    while let Some(module) = modules.next().unwrap() {
        // println!("Module: {}", module.object_file_name());
        let info = match pdb.module_info(&module).unwrap() {
            Some(info) => info,
            None => {
                // println!("  no module info");
                continue;
            }
        };
        let mut s = info.symbols().unwrap();
        while let Some(symbol) = s.next().unwrap() {
            // println!("{:?}", symbol);
            if let Ok(s) = symbol.parse() {
                match s {
                    pdb::SymbolData::Procedure(func) => {
                        // println!("{:x} {:?}", func.offset.offset, func.name);
                        if section_map.contains_key(&func.offset.section) {
                            let pa = func.offset.offset
                                + section_map[&func.offset.section].pointer_to_raw_data;
                            let va = func.offset.offset
                                + section_map[&func.offset.section].virtual_address;
                            map.insert(
                                pa,
                                (
                                    va,
                                    String::from_utf8(func.name.as_bytes().to_vec()).unwrap(),
                                ),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let mut sorted_funcs: Vec<(u32, (u32, String))> = vec![];
    for func in map.keys().sorted() {
        sorted_funcs.push((*func, (map[func].0, map[func].1.clone())));
    }

    for (fn_number, (function_pa, (function_va, function_name))) in sorted_funcs.iter().enumerate()
    {
        let func_bytes = match sorted_funcs.get(fn_number + 1) {
            Some((next_f_pa, _)) => &exe_bytes[*function_pa as usize..*next_f_pa as usize],
            _ => &exe_bytes[*function_pa as usize..],
        };
        let fn_size = guess_function_size(&func_bytes);
        if fn_size != 0 {
            result_functions.push(Function {
                rva: *function_va,
                data: &exe_bytes[*function_pa as usize..(*function_pa + fn_size) as usize],
                name: Some(function_name.clone()),
                fn_type: FunctionType::Exe,
            });
        }
    }

    // for f in result_functions {
    // println!("{}", f);
    // }

    Ok(result_functions)
}

pub fn get_functions_from_bin<'data>(
    exe_bytes: &'data [u8],
    min_fn_size: usize,
) -> Result<Vec<Function<'data>>, std::io::Error> {
    // funcs.append(&mut guess_smda_functions(filepath, &bytes));
    let mut result = get_exception_data_functions(&exe_bytes);
    // result.append(&mut get_exported_functions_goblin(&bytes));

    Ok(result
        .iter()
        .map(|x| x.clone())
        .filter(|f| f.data.len() >= min_fn_size)
        .collect())
}

pub fn get_functions_from_lib<'data>(lib_bytes: &'data [u8]) -> Vec<Function<'data>> {
    let mut result = vec![];

    match archive::Archive::parse(&lib_bytes) {
        Ok(archive) => {
            for (name, member, idx) in &archive.summarize() {
                // debug!("A {:?}\n B{:?}\n C{:?}\n", name, member, idx);
                let extracted = archive.extract(name, &lib_bytes).unwrap();

                if let Ok(coff_file) = goblin::pe::Coff::parse(extracted) {
                    // for section in coff_file.sections {
                    // println!("{:?}", section);
                    // }
                    // debug!("Coff file {}", name);
                    for (index, name, s) in coff_file.symbols.iter() {
                        if s.is_function_definition() {
                            debug!(
                                "Symbol nb {}, name : {:?} | {:?}",
                                index,
                                name,
                                s.name(&coff_file.strings)
                            );
                            // let fn_def = coff_file.symbols.aux_function_definition(index).unwrap();

                            // println!("idx {}, idx2 {}, Function size : {:x}, off {:x} type {:x}",index, fn_def.tag_index, fn_def.total_size, s.value, s.typ);
                            let sec_data = coff_file
                                .sections
                                .get(s.section_number as usize - 1)
                                .unwrap();
                            let symbol_bytes = &extracted[sec_data.pointer_to_raw_data as usize
                                ..(sec_data.pointer_to_raw_data + sec_data.size_of_raw_data)
                                    as usize];
                            for reloc in sec_data.relocations(extracted).unwrap().into_iter() {
                                // debug!("RELOC : {:?}", reloc);
                            }
                            debug!("data: {:?}", hex::encode(symbol_bytes));
                            debug!(
                                "data post: {:?}",
                                hex::encode(hash_single_func(symbol_bytes, false))
                            );
                            // debug!("\n\n");
                            // let hash = FuzzyHash::new(hash_single_func(symbol_bytes, false));
                            result.push(Function {
                                data: symbol_bytes,
                                name: Some(s.name(&coff_file.strings).unwrap().to_string()),
                                rva: 0,
                                fn_type: FunctionType::Lib,
                            })
                        }
                    }
                }

                //     println!("\n\n\n");

                //     // archive.
            }
        }
        Err(err) => println!("Err: {:?}", err),
    }

    // for member in x.members() {
    //     let m = x.get(member).unwrap();
    //     m.
    // }
    // let parsed_lib = goblin::pe::Coff::parse(lib_buf).unwrap();
    // for (index, name, symbol) in parsed_lib.symbols.iter() {
    //     if symbol.is_function_definition() {
    //         println!("{:?} {:x}", symbol.name, symbol.value);

    //     }
    // }

    result
}
