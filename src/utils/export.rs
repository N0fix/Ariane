use crate::functions_utils::search::rva_to_pa;

pub fn find_fn_name(start: u32, dll_bytes: &[u8]) -> Option<String> {
    let parsed_pe = goblin::pe::PE::parse(dll_bytes).unwrap();
    for export in &parsed_pe.exports {
        if let Some(pa) = rva_to_pa(&parsed_pe, export.rva as u32) {
            if pa == start as u32 {
                return Some(export.name.unwrap().to_string());
            }
        }
    }
    None
}
