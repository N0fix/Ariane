use std::collections::HashMap;

use fuzzyhash::FuzzyHash;
use log::{debug, error, info, log_enabled, Level};
use serde::{Deserialize, Serialize};

use super::sig_generation::FuzzyFunc;
use crate::{functions_utils::search::Function, sig::sig_generation::hash_functions};
use indicatif::ProgressBar;

#[derive(Serialize, Deserialize)]
pub struct Symbol {
    name: String,
    // pa: u32,
    rva: u32,
    score: u32,
}

fn compare_sigs(from: &Vec<FuzzyFunc>, with: &HashMap<String, String>) -> Vec<Symbol> {
    let mut symbols = vec![];
    // env_logger::init();
    let bar = ProgressBar::new(from.len() as u64);
    for f in from {
        bar.inc(1);

        for (f_name, hash) in with {
            // if let Some(function_name) = &f_dll.name {
            // if f_dll.hash.hash.to_string().len() < 10 {
            // println!("PA {:08x}  - {} {}", f.pa, function_name, f_dll.hash.hash.diff(&f.hash.hash, false));

            if let Ok(val) = FuzzyHash::compare(hash.to_owned(), f.hash.hash.to_string()) {
                if val > 25 {
                    debug!(
                        "RVA {:08x} - val {} - {} ({} {})",
                        f.rva, val, &f_name, &f.hash.hash, &hash
                    );
                    symbols.push(Symbol {
                        name: f_name.clone(),
                        // pa: f.pa,
                        rva: f.rva,
                        score: val,
                    });
                }
            } else {
                // println!("PA {:08x}  - {} - val {}", f.pa, function_name, "ERR COMPARE");
                // break;
            }
            // }
            // }
        }
    }

    bar.finish();
    symbols
}

pub fn compare(from: &Vec<FuzzyFunc>, with: &HashMap<String, String>) -> Vec<Symbol> {
    compare_sigs(&from, &with)
}
