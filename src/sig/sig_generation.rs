use std::fmt::Display;

use fuzzyhash::FuzzyHash;
use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, MasmFormatter, Mnemonic, OpKind};

use crate::{functions_utils::search::Function, utils::export::find_fn_name};

pub trait HashFn<T> {
    fn hash(bytes: &Vec<u8>) -> T;
    fn compare_hash(&self, with: &impl HashFn<T>) -> i32;
    fn get_hash(&self) -> &T;
}

pub struct Hash {
    // tlsh
    // pub hash: tlsh2::Tlsh128_1,
    pub hash: FuzzyHash,
}

pub struct FuzzyFunc {
    // pub pa: u32,
    pub rva: u32,
    pub hash: Hash,
    pub name: Option<String>,
}

impl Display for FuzzyFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(_name) = &self.name {
            write!(f, "{:#} {:?}", self.hash.hash, self.name)?;
        } else {
            write!(f, "{:#} ", self.hash.hash)?;
        }

        Ok(())
    }
}

impl HashFn<FuzzyHash> for FuzzyFunc {
    fn hash(bytes: &Vec<u8>) -> FuzzyHash {
        FuzzyHash::new(&bytes)
    }

    fn compare_hash(&self, with: &impl HashFn<FuzzyHash>) -> i32 {
        self.hash.hash.compare_to(with.get_hash()).unwrap() as i64 as i32
    }

    fn get_hash(&self) -> &FuzzyHash {
        &self.hash.hash
    }
}

const MIN_FUNC_SZ: u8 = 20;

/// Uses ssdeep to hash the function.
/// Given bytes gets disassembled to avoid anything that has a relative offset to get hashed, e.g:
/// ```
/// lea rax, off_14006580
/// ```
/// This function also stop hashing as soon as it encounters two consecutive `ud2` or `int3`.
pub fn hash_single_func(bytes: &[u8], verbose: bool) -> Vec<u8> {
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
        match instruction.mnemonic() {
            Mnemonic::Call => {
                result.push(0xe8);
                continue;
            }
            Mnemonic::Jmp => {
                result.push(0xe9);
                continue;
            }
            _ => {}
        };
        if instruction.op1_kind() == OpKind::Memory && instruction.ip_rel_memory_address() != 0 {
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

pub fn hash_functions(functions: &Vec<Function>) -> Vec<FuzzyFunc> {
    let mut result = vec![];

    for f in functions {
        // if f.end_pa < f.start_pa || f.end_pa - f.start_pa < MIN_FUNC_SZ.into() {
        //     continue;
        // }

        // end_pa MUST be next exported func if it has one
        let data = hash_single_func(f.data, false);

        if data.len() < MIN_FUNC_SZ.into() {
            continue;
        }

        let hash = FuzzyHash::new(&data);

        // let fn_name = match f.name.clone() {
        //     Some(name) => Some(name),
        //     None => find_fn_name(f.start_pa, file_bytes),
        // };
        result.push(FuzzyFunc {
            // pa: f.start_pa,
            rva: f.rva,
            // data: f.data,
            hash: Hash { hash: hash },
            name: f.name.clone(),
        });
    }

    result
}
