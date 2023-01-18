use anyhow::Result;
use clap::Parser;
use rdns_config::load_txt_config;

mod cli;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Check { zone_file } => {
            load_txt_config(zone_file)?;
        }
    }

    Ok(())
}
