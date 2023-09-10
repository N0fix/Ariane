use clap::{Parser, Subcommand};
use std::path::PathBuf;
mod commands;

use crate::commands::download::download_subcommand;
use crate::commands::info::info_subcommand;
use crate::commands::recover::recover_subcommand;

#[derive(Subcommand, Debug)]
enum SubCommand {
    /// Print recognized dependencies
    Info(InfoArgs),
    /// Download and extract recognized dependencies to target directory
    Download(DownloadArgs),
    /// Try to recover symbols
    Recover(RecoverArgs),
}

#[derive(Parser, Debug)]
#[clap(version)]
pub struct Arguments {
    #[clap(subcommand)]
    cmd: SubCommand,
}

#[derive(Parser, Debug)]
pub struct InfoArgs {
    pub target: String,
}

#[derive(Parser, Debug)]
pub struct DownloadArgs {
    pub target: String,
    pub dest_directory: PathBuf,
}

#[derive(Parser, Debug)]
pub struct RecoverArgs {
    pub target: String,
    #[clap(short, long, required = false)]
    input_functions_file: Option<PathBuf>,
    // #[clap(required = false)]
    // pub dest_directory: Option<PathBuf>,
    #[clap(required = true)]
    result_file: String,
}

fn main() -> Result<(), std::io::Error> {
    env_logger::init();

    let args = Arguments::parse();
    println!("{args:#?}");
    match args.cmd {
        SubCommand::Info(subcommand_args) => {
            return info_subcommand(&subcommand_args);
        }
        SubCommand::Download(subcommand_args) => {
            return download_subcommand(&subcommand_args);
        }
        SubCommand::Recover(subcommand_args) => {
            return recover_subcommand(&subcommand_args);
        }
    }
}
