use regex::bytes::Regex;
use std::io::{Cursor, Write};

/// If not found, considers that hash corresponds to the latest rustc (no tag available yet). Returns None in this case.
pub fn find_tag_from_hash(hash: &str) -> Option<String> {
    let tag_regex = Regex::new(r##"href="/rust-lang/rust/releases/tag/(?<tag>[0-9\.]+)"##).unwrap();

    // curl -s https://github.com/rust-lang/rust/branch_commits/9c20b2a8cc7588decb6de25ac6a7912dcef24d65
    let url = format!(
        "https://github.com/rust-lang/rust/branch_commits/{:#}",
        hash
    );
    let mut result = None;
    let client = reqwest::blocking::Client::new();
    // According to https://github.com/s0md3v/Zen :
    // "Github allows 60 unauthenticated requests per hour". This should be way enough for a single user, but might reach a limit in CTF.
    let mut response = client.get(&url).send().unwrap();
    let mut content = Cursor::new(response.bytes().unwrap());
    let ca = tag_regex.captures_iter(content.get_ref());
    // let mut latest_tag = None;
    for c in ca {
        let v = String::from_utf8(c.name("tag").unwrap().as_bytes().to_vec()).unwrap();
        result = Some(v);
    }

    result
}

pub fn finc_compiler_version(content: &Vec<u8>) -> Option<String> {
    let version_regex = Regex::new(r"rustc/(?<hash>[a-z0-9]+)").unwrap();

    // let x = re.captures_iter(content.as_ref());//.collect();
    for c in version_regex.captures_iter(content.as_ref()) {
        let v = String::from_utf8(c.name("hash").unwrap().as_bytes().to_vec()).unwrap();
        return Some(v);
    }

    None
}
