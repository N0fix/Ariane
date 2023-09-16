use regex::bytes::Regex;
use std::{
    io::{Cursor, Write},
    path::Path,
};

#[derive(Clone)]
pub struct CommitHash {
    hash: String,
    tag: Option<String>,
}

impl CommitHash {
    pub fn commit_hash_to_string(&self) -> &String {
        &self.hash
    }

    /// Blocking network request to github to try to find latest tag related to the commit hash.
    /// Returns the tag if found, or latest rustc version if not found (considering the hash must
    /// belong to an unreleased tag).
    pub fn search_rustc_version(&self) -> Option<String> {
        match search_rustc_version_from_commit(&self.hash) {
            Some(version) => Some(version),
            None => get_latest_rustc_version(),
        }
    }
}

#[derive(Clone)]
///```
/// let compiler_version = String::new();
///
/// if let Some(compiler_info) = RustcInformation::from_file(&Path::new(&args.file))? {
///     let rustc_commit_hash = compiler_info.get_commit_hash();
///     println!("{}", rustc_commit_hash.commit_hash_to_string());
///     compiler_version = rustc_commit_hash.search_rustc_version().expect("Could not find rustc version from your target !");
/// }
/// ```
pub struct RustcInformation {
    hash: CommitHash,
}

impl RustcInformation {
    pub fn get_commit_hash(&self) -> &CommitHash {
        &self.hash
    }

    /// Searches rustc commit hash from a file on disk.
    pub fn from_file(filepath: &Path) -> Result<Option<RustcInformation>, std::io::Error> {
        let content = std::fs::read(&filepath)?;

        Ok(RustcInformation::from_buffer(&content))
    }

    /// Searches rustc commit hash from a buffer representing a file on disk.
    pub fn from_buffer(buffer: &Vec<u8>) -> Option<RustcInformation> {
        let version_regex = Regex::new(r"rustc/(?<hash>[a-z0-9]+)").unwrap();

        // let x = re.captures_iter(content.as_ref());//.collect();
        for c in version_regex.captures_iter(buffer.as_ref()) {
            let v = String::from_utf8(c.name("hash").unwrap().as_bytes().to_vec()).unwrap();
            return Some(RustcInformation {
                hash: CommitHash { hash: v, tag: None },
            });
        }

        None
    }
}

fn search_rustc_version_from_commit(hash: &str) -> Option<String> {
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
    let response = client.get(&url).send().unwrap();
    let content = Cursor::new(response.bytes().unwrap());
    let ca = tag_regex.captures_iter(content.get_ref());
    for c in ca {
        let v = String::from_utf8(c.name("tag").unwrap().as_bytes().to_vec()).unwrap();
        result = Some(v);
    }

    result
}

fn get_latest_rustc_version() -> Option<String> {
    let tag_regex = Regex::new(r##"/rust-lang/rust/releases/tag/(?<tag>[0-9\.]+)"##).unwrap();
    let url = String::from("https://github.com/rust-lang/rust/tags");

    let mut result = None;
    let client = reqwest::blocking::Client::new();
    // According to https://github.com/s0md3v/Zen :
    // "Github allows 60 unauthenticated requests per hour". This should be way enough for a single user, but might reach a limit in CTF.
    let response = client.get(&url).send().unwrap();
    let content = Cursor::new(response.bytes().unwrap());
    let ca = tag_regex.captures_iter(content.get_ref());
    // let mut latest_tag = None;
    for c in ca {
        let v = String::from_utf8(c.name("tag").unwrap().as_bytes().to_vec()).unwrap();
        result = Some(v);
        break;
    }

    result
}
