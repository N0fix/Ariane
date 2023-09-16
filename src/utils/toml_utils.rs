use toml_edit::{Document, Item, Table};

pub fn add_array(doc: &mut Document, node: &str, key: &str, item: &Item) {
    let tbl = doc.as_table_mut();

    // If node does not exist, create it
    if let None = tbl.get_key_value_mut(node) {
        tbl.insert(node, Item::Table(Table::new()));
    }

    let (_, lib_item) = tbl.get_key_value_mut(node).unwrap();
    let node_tbl = lib_item.as_table_like_mut().unwrap();
    // Remove existing key
    node_tbl.remove(key);
    node_tbl.insert(key, item.clone());
}
