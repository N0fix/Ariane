[![Rust](https://github.com/N0fix/Ariane/actions/workflows/rust.yml/badge.svg)](https://github.com/N0fix/Ariane/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)


## Warning

**This tool is currently in an experimental phase and should not be considered as complete and accurate.**

**It is heavely inspired by [Cerberus](https://github.com/h311d1n3r/Cerberus/tree/main) and implement the same principles, but for PE files.**

## Usage

First, you need to provide a list of functions from your target. Scripts to extract them from IDA and convert them to the correct format are available under `tools/IDA_extract_functions`.

This list of functions should be in JSON format and have the following structure:

```json
{
  "functions": [
    {
      "name": "sub_140001000",
      "start": 4096,
      "end": 4230
    },
    [... more entries ...]
  ]
}
```

Next, pass this JSON file as an argument along with your target and specify an output file.

```
ariane.exe -i functions_list.json no_symbols_target.exe resolved_symbols.json
```

The output file will be in JSON format and will contain resolved symbols, along with their physical addresses (PA) and relative virtual addresses (RVA). You can find a script under tools/output_to_idc.py that can generate an IDA IDC script. This IDC script will rename all resolved symbols to aid in your analysis.

## FAQ

### How does this work ?

This tool searches for your target's dependencies by looking for specific strings. It identifies the version of rustc used to compile your target and compiles all dependencies with it, including symbols. These symbols are used to identify functions and fuzzy-hash them. This hash is then compared to hashed functions from your target.

### Why can't you produce a pdb file with symbols attached?

Generating a PDB file is not an easy task and, as far as I know, requires heavy dependencies (LLVM).

### How is this different from [Cerberus](https://github.com/h311d1n3r/Cerberus/tree/main) ?

This project focuses on recovering symbols from PE files only, specifically for Rust executables. Cerberus aims at recovering symbols from ELF files for both Golang and Rust. Cerberus plans to support PE files in the near future. If this tool does not work for you, please test Cerberus !


## Thanks

@ h311d1n3r
