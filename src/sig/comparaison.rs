use fuzzyhash::FuzzyHash;
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
                            "PA {:08x} - val {} - {} ({} {})",
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
    }

    symbols
}

pub fn compare(
    from: &Vec<FuzzyFunc>,
    with: &Vec<FuzzyFunc>
) -> Vec<Symbol> {
    compare_sigs(&from, &with)
}
