use crates_io_api::{CrateResponse, SyncClient};
use log::{debug, error, info, log_enabled, Level};
use regex::bytes::Regex;
use semver::Version;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Krate {
    pub name: String,
    pub version: Version,
    download_url: String,
    features: Vec<String>,
    is_accurate: bool,
    metadata: Option<CrateResponse>,
}

impl Display for Krate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.name, self.version)
    }
}

#[derive(Debug)]
pub enum KrateError {
    CursorError(std::io::Error),
    DownloadError(reqwest::Error),
    FileCreationError(std::io::Error),
    NoMetadataError(crates_io_api::Error),
    NonExistantVersion,
}

impl Krate {
    pub fn new(name: &str, version: Version) -> Krate {
        Krate {
            name: name.to_string(),
            version: version,
            download_url: String::new(),
            features: vec![],
            is_accurate: false,
            metadata: None,
        }
    }

    pub fn from_name(name: &str) -> Result<Krate, KrateError> {
        let metadata = match Krate::get_metadata_from_crates_api_from_name(name) {
            Ok(m) => m,
            Err(e) => return Err(KrateError::NoMetadataError(e)),
        };
        let version = metadata.versions.last().unwrap();
        Ok(Krate::new(name, Version::parse(&version.num).unwrap()))
    }

    pub fn new_with_remote_info(name: &str, version: Version) -> Krate {
        let mut k = Krate::new(name, version);

        k.fill_information_from_crates_api();

        k
    }

    /// Retrives krate metadata from crates.io.
    /// ⚠️ This makes a blocking request to crates.io/api ⚠️
    pub fn get_krate_meta(&mut self) -> Option<CrateResponse> {
        if !self.is_accurate {
            self.fill_information_from_crates_api();
        }

        self.metadata.clone()
    }

    pub fn download(&mut self, dest_dir: &Path) -> Result<PathBuf, KrateError> {
        debug!(
            "Downloading {} to {:?}",
            self.name,
            &dest_dir.to_string_lossy()
        );
        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
            return Err(KrateError::FileCreationError(e));
        };

        let reqwest_client = reqwest::blocking::Client::new();

        let response = match reqwest_client.get(self.get_download_url()?).send() {
            Ok(response) => response,
            Err(e) => return Err(KrateError::DownloadError(e)),
        };

        let tarball_path = dest_dir.clone().join(format!("{:#}.tar.gz", self.name));
        let mut tarball_file = match std::fs::File::create(&tarball_path) {
            Ok(tarball) => tarball,
            Err(e) => return Err(KrateError::FileCreationError(e)),
        };
        let response_content = match response.bytes() {
            Ok(b) => b,
            Err(e) => return Err(KrateError::DownloadError(e)),
        };
        let mut content = std::io::Cursor::new(response_content);

        if let Err(e) = std::io::copy(&mut content, &mut tarball_file) {
            return Err(KrateError::FileCreationError(e));
        };

        Ok(tarball_path)
    }

    fn get_metadata_from_crates_api_from_name(name: &str) -> Result<CrateResponse, crates_io_api::Error> {
        let client = SyncClient::new(
            "Ariane (https://github.com/N0fix/Ariane)",
            std::time::Duration::from_millis(1_0000),
        )
        .unwrap();

        client.get_crate(name)
    }

    fn get_metadata_from_crates_api(&self) -> Result<CrateResponse, crates_io_api::Error> {
        Krate::get_metadata_from_crates_api_from_name(&self.name.as_str())
    }

    fn filter_features(&mut self, version_meta: &crates_io_api::Version) {
        let set_a: HashSet<String> = version_meta.features.keys().map(|x| x.clone()).collect();
        let set_b: HashSet<String> = self.features.clone().iter().map(|x| x.clone()).collect();
        self.features = set_a.intersection(&set_b).map(|s| s.clone()).collect();
    }

    /// ⚠️ This makes a blocking request to crates.io/api ⚠️
    fn fill_information_from_crates_api(&mut self) -> Result<&Krate, KrateError> {
        if self.is_accurate {
            return Ok(self);
        }

        let metadata = match self.get_metadata_from_crates_api() {
            Ok(meta) => meta,
            Err(e) => return Err(KrateError::NoMetadataError(e)),
        };

        self.metadata = Some(metadata.clone());

        let v = match metadata
            .versions
            .iter()
            .find(|v| v.num == self.version.to_string())
        {
            Some(v) => v,
            None => return Err(KrateError::NonExistantVersion),
        };

        self.filter_features(&v);
        self.download_url = format!("https://crates.io{}", v.dl_path);

        self.is_accurate = true;

        Ok(self)
    }

    /// Gets a list of potential features used. This is a list with numerus potential false positives.
    /// Many items of this list might not even exist as a feature.
    /// If you want an accurate list of features, use `get_features()`.
    pub fn get_features_raw(&self) -> &Vec<String> {
        &self.features
    }

    /// Retrieves download url.
    /// ⚠️ This makes a blocking request to crates.io/api ⚠️
    pub fn get_download_url(&mut self) -> Result<&str, KrateError> {
        if !self.is_accurate {
            self.fill_information_from_crates_api()?;
        }

        Ok(&self.download_url)
    }

    /// Retrives potential features used.
    /// ⚠️ This makes a blocking request to crates.io/api ⚠️
    pub fn get_features(&mut self) -> Result<&Vec<String>, KrateError> {
        if !self.is_accurate {
            self.fill_information_from_crates_api()?;
        }

        Ok(&self.features)
    }

    pub fn as_string(&self) -> String {
        String::from(format!(
            "{}-{}.{}.{}",
            self.name, self.version.major, self.version.minor, self.version.patch
        ))
    }
}

#[derive(Clone)]
///
/// ```
/// let mut deps: Dependencies = Dependencies::from_buffer(&bytes);
///
/// for cr in deps.get_dependencies_mut() {
///     println!("{:?}", cr);
/// }
/// ```
pub struct Dependencies {
    krates: Vec<Krate>,
}

impl Dependencies {
    pub fn get_dependencies(&self) -> &Vec<Krate> {
        &self.krates
    }

    pub fn get_dependencies_mut(&mut self) -> &mut Vec<Krate> {
        &mut self.krates
    }

    pub fn from_file(filepath: &Path) -> Result<Dependencies, std::io::Error> {
        let content = std::fs::read(&filepath)?;

        Ok(Dependencies::from_buffer(&content))
    }

    pub fn from_buffer(buffer: &Vec<u8>) -> Dependencies {
        let mut map = HashMap::<String, Version>::new();
        let re = Regex::new(r"cargo.registry.src.[^\\\/]+.(?<cratename>[^\\\/]+)").unwrap();
        let re_sources = Regex::new(
            r"cargo.registry.src.[^\\\/]+.([^\\\/]+[^\\\/])+.([^\\\/]+)+.(?<sources>[^\\\/].*).rs",
        )
        .unwrap();
        // let x = re.captures_iter(content.as_ref());//.collect();
        let ca = re.captures_iter(buffer.as_ref());
        let ca_sources = re_sources.captures_iter(buffer.as_ref());
        let mut features = vec![];

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
        for c in ca_sources {
            if let Some(sources) = c.name("sources") {
                let sources = String::from_utf8(sources.as_bytes().to_vec()).unwrap();
                if sources.contains("\\") {
                    for s in sources.split("\\") {
                        features.push(s.to_string());
                    }
                } else {
                    features.push(sources);
                }
            }
        }

        Dependencies {
            krates: map
                .iter()
                .map(|(name, version)| Krate {
                    name: name.to_owned(),
                    version: version.to_owned(),
                    download_url: String::new(),
                    features: features.clone(),
                    is_accurate: false,
                    metadata: None,
                })
                .collect(),
        }
    }
}
