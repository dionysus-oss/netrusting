use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Checks a zone file
    Check {
        /// The zone file to check
        #[arg(short, long)]
        zone_file: PathBuf,
    },
}
