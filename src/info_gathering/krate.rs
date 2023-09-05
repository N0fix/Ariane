use regex::bytes::Regex;
use semver::Version;
use std::{collections::HashMap, fmt::Display};

#[derive(Clone)]
pub struct Krate {
    pub name: String,
    pub version: Version,
}

impl Display for Krate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.name, self.version)
    }
}

impl Krate {
    pub fn as_string(&self) -> String {
        String::from(format!(
            "{}-{}.{}.{}",
            self.name, self.version.major, self.version.minor, self.version.patch
        ))
    }
}

pub fn find_deps(content: &Vec<u8>) -> Vec<Krate> {
    let mut map = HashMap::<String, Version>::new();
    let re = Regex::new(r"cargo.registry.src.[^\\\/]+.(?<cratename>[^\\\/]+)").unwrap();
    // let x = re.captures_iter(content.as_ref());//.collect();
    let ca = re.captures_iter(content.as_ref());

    for c in ca {
        // println!("{:?}",c);
        if let Some(cratename) = c.name("cratename") {
            let crate_string = String::from_utf8(cratename.as_bytes().to_vec()).unwrap();
            let (name, version) = crate_string.rsplit_once('-').unwrap();
            let splited_version: Vec<u64> = version
                .split('.')
                .into_iter()
                .map(|x| x.parse::<u64>().unwrap())
                .collect();
            let version = Version::new(
                *splited_version.get(0).unwrap(),
                *splited_version.get(1).unwrap(),
                *splited_version.get(2).unwrap(),
            );
            map.insert(name.to_string(), version);
        }
    }

    map.iter()
        .map(|(name, version)| Krate {
            name: name.to_owned(),
            version: version.to_owned(),
        })
        .collect()
}
