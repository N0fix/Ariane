use std::fmt::Display;

use fuzzyhash::FuzzyHash;
use iced_x86::{Decoder, MasmFormatter, DecoderOptions, Instruction, Mnemonic, OpKind, Formatter};

use crate::{functions_utils::search::Function, utils::export::find_fn_name};

pub struct Hash {
    // tlsh
    // pub hash: tlsh2::Tlsh128_1,
    pub hash: FuzzyHash,
}

pub struct FuzzyFunc {
    pub pa: u32,
    pub rva: u32,
    pub hash: Hash,
    pub name: Option<String>,
}

impl Display for FuzzyFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(_name) = &self.name {
            write!(f, "{:08x} {:?}", self.pa, self.name)?;
        }

        Ok(())
    }
}

/// Uses ssdeep to hash the function.
/// Given bytes gets disassembled to avoid anything that has a relative offset to get hashed, e.g:
/// ```
/// lea rax, off_14006580
/// ```
/// This function also stop hashing as soon as it encounters two consecutive `ud2` or `int3`.
fn hash_single_func(bytes: &[u8], verbose: bool) -> Vec<u8> {
    let mut result = vec![];
    // println!("HASHING A SINGLE FN");
    let mut decoder = Decoder::new(64, bytes, DecoderOptions::NONE);
    let mut formatter = MasmFormatter::new();
    let mut output = String::new();

    let mut fn_end: u32 = 0;

    // Initialize this outside the loop because decode_out() writes to every field
    let mut instruction = Instruction::default();
    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);
        output.clear();
        formatter.format(&instruction, &mut output);
        if instruction.mnemonic() == Mnemonic::Call
            || (instruction.op1_kind() == OpKind::Memory
                && instruction.ip_rel_memory_address() != 0)
        {
            continue;
        }

        match instruction.mnemonic() {
            Mnemonic::Ud2 | Mnemonic::Int3 => {
                fn_end += 1;
            }
            _ => {
                fn_end = 0;
            }
        };
        if fn_end == 2 {
            break;
        }
        let start_index = (instruction.ip()) as usize;
        let instr_bytes = &bytes[start_index..start_index + instruction.len()];
        for b in instr_bytes.iter() {
            if verbose {
                print!("{:02X}", b);
            }
            result.push(*b);
        }
        if verbose {
            println!(
                " {} {} {}",
                output,
                instruction.op1_kind() != OpKind::Memory,
                instruction.ip_rel_memory_address() == 0
            );
        }
    }
    if verbose {
        println!("{}", hex::encode(&result));
    }
    result
}

pub fn hash_functions(file_bytes: &[u8], functions: &Vec<Function>) -> Vec<FuzzyFunc> {
    let mut result = vec![];

    for f in functions {
        // end_pa MUST be next exported func if it has one
        let data = hash_single_func(
            &file_bytes[f.start_pa as usize..f.end_pa as usize],
            false //f.name.clone().unwrap() == String::from("aes::ni::aes128::expand_key") || f.start_pa == 0x1FC0,
        );
        let hash = FuzzyHash::new(&data);

        result.push(FuzzyFunc {
            pa: f.start_pa,
            rva: f.start_rva,
            hash: Hash { hash: hash },
            name: find_fn_name(f.start_pa, file_bytes),
        });
    }

    result
}
