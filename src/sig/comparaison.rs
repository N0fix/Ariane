


use serde::{Serialize, Deserialize};

use crate::{functions_utils::search::Function, sig::sig_generation::hash_functions};

use super::sig_generation::FuzzyFunc;


#[derive(Serialize, Deserialize)]
pub struct Symbol {
    name: String,
    pa: u32,
    rva: u32,
    score: u32,
}


fn compare_sigs(from: &Vec<FuzzyFunc>, with: &Vec<FuzzyFunc>) -> Vec<Symbol> {
    let mut symbols = vec![];

    for f in from {
        for f_dll in with {
            if let Some(function_name) = &f_dll.name {
                // if f_dll.hash.hash.to_string().len() < 10 {
                // println!("PA {:08x}  - {} {}", f.pa, function_name, f_dll.hash.hash.diff(&f.hash.hash, false));
                if let Some(val) = f_dll.hash.hash.compare_to(&f.hash.hash) {
                    if val > 25 {
                        println!(
                            "PA {:08x}  - val {} - {} ({} {})",
                            f.pa, val, function_name, f.hash.hash, f_dll.hash.hash
                        );
                        symbols.push(Symbol {
                            name: function_name.clone(),
                            pa: f.pa,
                            rva: f.rva,
                            score: val,
                        });
                    }
                } else {
                    // println!("PA {:08x}  - {} - val {}", f.pa, function_name, "ERR COMPARE");
                    // break;
                }
                // }
            }
        }
        // let hash = FuzzyHash::new(&dll_bytes[f.start_pa as usize..f.end_pa as usize]);
        // for (addr, (digest, _)) in map.iter() {
        //     if let Ok(val) = FuzzyHash::compare(digest.to_string(), hash.to_string()) {
        //         let fn_name = find_fn_name(f.start_pa, dll_bytes);
        //         if let Some(name) = fn_name {
        //             if val > 20 {
        //                 println!(
        //                     "val {}, {} offset exe {:x} offset dll {:x} ({} {})",
        //                     val, name, addr, f.start_pa, digest, hash
        //                 );
        //             }
        //         }
        //     }
        // }
    }

    symbols
}

pub fn compare(
    file_bytes: &[u8],
    functions: &Vec<Function>,
    dll_bytes: &[u8],
    dll_functions: &Vec<Function>,
) -> Vec<Symbol> {
    println!(
        "Comparing {} funcs to {}",
        functions.len(),
        dll_functions.len()
    );
    println!("Generating hashes");
    println!("Gen dll");
    let dll_hash_fn = hash_functions(dll_bytes, dll_functions);
    println!("Gen target");
    let normal_hash_fn = hash_functions(file_bytes, functions);

    println!("End hashes generation");
    let sym = compare_sigs(&normal_hash_fn, &dll_hash_fn);
    return sym;
}
