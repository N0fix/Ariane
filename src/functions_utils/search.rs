use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;
use std::{fs, path::Path};

use goblin::pe::section_table::SectionTable;
use itertools::Itertools;
use pdb::FallibleIterator;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct Function {
    pub start_rva: u32,
    pub end_rva: u32,
    pub start_pa: u32,
    pub end_pa: u32,
    pub name: Option<String>,
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Virtual {:x}-{:x} | Physical {:x}-{:x} | {:?}",
            self.start_rva, self.end_rva, self.start_pa, self.end_pa, self.name
        )
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

fn get_exported_functions_goblin(file: &[u8]) -> Vec<Function> {
    let mut functions = vec![];
    let mut function_starts = vec![];
    let parsed_pe = goblin::pe::PE::parse(file).unwrap();
    for export in &parsed_pe.exports {
        function_starts.push((export.rva, export.name.to_owned()));
    }

    function_starts.sort();

    let mut previous = 0;
    for (i, function) in function_starts.iter().enumerate() {
        let (start, fn_name) = *function;
        if let Some(start_pa) = rva_to_pa(&parsed_pe, start as u32) {
            assert!(start >= previous);
            previous = start;
            let mut end = 0;

            let func_bytes = match function_starts.get(i + 1) {
                // max is next exported function addr
                Some((next_fn, _)) => {
                    if let Some(end_pa) = rva_to_pa(&parsed_pe, *next_fn as u32) {
                        &file[start_pa as usize..end_pa as usize]
                    } else {
                        &file[start_pa as usize..]
                    }
                }

                // or no max if latest export
                None => &file[start_pa as usize..],
            };

            let mut padding_bytes = 0;
            let mut func_size = 0;
            for b in func_bytes.iter() {
                if *b == 0xCC {
                    padding_bytes += 1;
                } else {
                    padding_bytes = 0;
                }

                if padding_bytes > 2 {
                    break;
                }

                func_size += 1;
            }
            end = start + func_size;

            if start == end || start >= (end - padding_bytes) {
                continue;
            }
            let name = match fn_name {
                Some(name) => Some(name.to_owned()),
                None => None,
            };
            if let Some(end_pa) = rva_to_pa(&parsed_pe, end as u32) {
                functions.push(Function {
                    start_rva: start as u32,
                    end_rva: (end - padding_bytes) as u32,
                    start_pa: start_pa,
                    end_pa: end_pa,
                    name: name,
                });
            }
        }
    }

    functions
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
                            start_rva: f.begin_address,
                            end_rva: f.end_address,
                            start_pa: start_pa,
                            end_pa: end_pa,
                            // todo : search for symbols in dwarf
                            name: None,
                        });
                    }
                }
            }
        }
    }

    functions
}

// fn guess_smda_functions(filepath: &Path, file_content: &[u8]) -> Vec<Function> {
//     let mut fns = vec![];
//     let parsed_pe = goblin::pe::PE::parse(file_content).unwrap();
//     let file =
//         smda::Disassembler::disassemble_file(filepath.to_str().unwrap(), true, true).unwrap();
//     for (_, func) in file.get_functions().unwrap().iter() {
//         let end = func
//             .get_instructions()
//             .unwrap()
//             .iter()
//             .map(|x| x.offset)
//             .max()
//             .unwrap();

//         if end > func.offset && end - func.offset > VALID_MIN_BYTES_FUNCTION_TRESHOLD.into() {
//             if let Some(start_pa) = rva_to_pa(&parsed_pe, (func.offset - file.base_addr) as u32) {
//                 if let Some(end_pa) = rva_to_pa(&parsed_pe, end as u32) {
//                     fns.push(Function {
//                         start_rva: (func.offset - file.base_addr) as u32,
//                         end_rva: (end - file.base_addr) as u32,
//                         start_pa: start_pa,
//                         end_pa: end_pa,
//                         name: None,
//                     })
//                 }
//             }
//         }
//     }

//     fns.sort();
//     fns
// }

pub fn get_functions(filepath: &Path, pdb_path: Option<&Path>) -> Vec<Function> {
    let mut funcs = vec![];
    let bytes = match fs::read(filepath) {
        Ok(b) => b,
        Err(e) => {
            writeln!(std::io::stderr(), "Could not read file : {}", e);
            return funcs;
        }
    };

    if let Some(pdb_path) = pdb_path {
        match pdb_path.exists() {
            true => return enumerate_pdb_symbols(&bytes, pdb_path),
            false => panic!("No pdb found for {:?}!", pdb_path),
        }
    }

    // funcs.append(&mut guess_smda_functions(filepath, &bytes));
    funcs.append(&mut get_exception_data_functions(&bytes));
    funcs.append(&mut get_exported_functions_goblin(&bytes));

    funcs
        .iter()
        .map(|x| x.clone())
        .filter(|f| f.end_pa - f.start_pa > VALID_MIN_BYTES_FUNCTION_TRESHOLD)
        .collect()
}

fn enumerate_pdb_symbols(executable_buf: &[u8], pdb_path: &Path) -> Vec<Function> {
    println!("PDB found, enumerating symbols");
    let parsed_pe = goblin::pe::PE::parse(executable_buf).unwrap();
    let mut section_map = HashMap::<u16, SectionTable>::new();
    for (i, section) in parsed_pe.sections.iter().enumerate() {
        section_map.insert(i as u16 + 1, section.clone());
    }
    let file = std::fs::File::open(pdb_path).unwrap();
    let mut pdb = pdb::PDB::open(file).unwrap();

    let symbol_table = pdb.global_symbols().unwrap();
    // let address_map = pdb.address_map().unwrap();

    let mut symbols = symbol_table.iter();

    let mut map = HashMap::<u32, (u32, String)>::new();
    // dbg!(&section_map);
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
            // pdb::SymbolData::Procedure(func) => {
            //     println!("{:?}", func);
            //     if section_map.contains_key(&func.offset.section) {
            //         let pa =
            //             func.offset.offset + section_map[&func.offset.section].pointer_to_raw_data;
            //         let va = func.offset.offset + section_map[&func.offset.section].virtual_address;
            //         map.insert(
            //             pa,
            //             (
            //                 va,
            //                 String::from_utf8(func.name.as_bytes().to_vec()).unwrap(),
            //             ),
            //         );
            //     }
            // }
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
        // walk_symbols(info.symbols())?;
    }

    let mut sorted_funcs: Vec<(u32, (u32, String))> = vec![];
    for func in map.keys().sorted() {
        sorted_funcs.push((*func, (map[func].0, map[func].1.clone())));
    }

    for (fn_number, (function_pa, (function_va, function_name))) in sorted_funcs.iter().enumerate()
    {
        let func_bytes = match sorted_funcs.get(fn_number + 1) {
            Some((next_f_pa, _)) => &executable_buf[*function_pa as usize..*next_f_pa as usize],
            _ => &executable_buf[*function_pa as usize..],
        };
        let fn_size = guess_function_size(&func_bytes);
        if fn_size != 0 {
            result_functions.push(Function {
                start_rva: *function_va,
                end_rva: *function_va + fn_size,
                start_pa: *function_pa,
                end_pa: *function_pa + fn_size,
                name: Some(function_name.clone()),
            });
        }
    }

    // for f in result_functions {
    // println!("{}", f);
    // }

    result_functions
}
