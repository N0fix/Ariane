use ariane::{
    compilation::compile,
    info_gathering::{
        compiler::{CommitHash, RustcInformation},
        krate::Dependencies,
    },
};
use log::{debug, error, info, log_enabled, Level};

use crate::InfoArgs;

pub fn info_subcommand(args: &InfoArgs) -> Result<(), std::io::Error> {
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
        "Compiler version: {} ({})\n",
        rustc_version,
        rustc_commit_hash.commit_hash_to_string()
    );

    let deps = Dependencies::from_buffer(&bytes);
    for dep in deps.get_dependencies() {
        println!("{:#}", dep);
    }

    Ok(())
}
