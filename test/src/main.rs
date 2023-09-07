use exe::pe::{PE, VecPE};
use exe::types::{ImportDirectory, ImportData, CCharString};

fn main() {
    let image = VecPE::from_disk_file("test/compiled.exe").unwrap();
    let import_directory = ImportDirectory::parse(&image).unwrap();
    
    for descriptor in import_directory.descriptors {
       println!("Module: {}", descriptor.get_name(&image).unwrap().as_str().unwrap());
       println!("Imports:");
    
       for import in descriptor.get_imports(&image).unwrap() {
          match import {
             ImportData::Ordinal(x) => println!("   #{}", x),
             ImportData::ImportByName(s) => println!("   {}", s)
          }
       }
    }
}