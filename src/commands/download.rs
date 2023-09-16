use crate::DownloadArgs;
use ariane::info_gathering::{
    compiler::{CommitHash, RustcInformation},
    krate::Dependencies,
};
use flate2::read::GzDecoder;
use log::{debug, error, info, log_enabled, Level};
use tar::Archive;

pub fn download_subcommand(args: &DownloadArgs) -> Result<(), std::io::Error> {
    let bytes = std::fs::read(&args.target)?;

    let rustc_information = RustcInformation::from_buffer(&bytes).expect(&format!(
        "Could not find rustc version on target file : {:?}",
        &args.target
    ));
    let rustc_commit_hash: CommitHash = rustc_information.get_commit_hash().to_owned();
    let rustc_version = rustc_commit_hash
        .search_rustc_version()
        .expect("Could not find rustc version from your target !");

    println!(
        "Compiler version: {} (commit {})\n",
        rustc_version,
        rustc_commit_hash.commit_hash_to_string()
    );

    let mut deps = Dependencies::from_buffer(&bytes);
    for dep in deps.get_dependencies_mut() {
        println!("Downloading {:#}", dep);
        let targz_path = match dep.download(&args.dest_directory) {
            Ok(path) => path,
            Err(e) => {
                error!("Could not download crate");
                continue;
            }
        };
        let mut archive = Archive::new(GzDecoder::new(std::fs::File::open(&targz_path).unwrap()));
        archive.unpack(&args.dest_directory)?;
        println!(
            "Extracted to {:#}{:#}",
            args.dest_directory.to_string_lossy(),
            dep
        );
    }

    Ok(())
}
