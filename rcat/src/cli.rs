use clap::{Parser, Subcommand};
use std::path::Path;
use std::ops::RangeInclusive;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Connect to a server
    Connect {
        host: String,

        #[arg(long, short, value_parser = port_in_range)]
        port: u16,

        #[arg(long, default_value_t = false)]
        tls: bool,

        #[arg(long, value_parser = valid_path)]
        ca: Option<String>,

        #[arg(long, default_value_t = false)]
        udp: bool,

        #[arg(long, value_parser = port_in_range)]
        listen_port: Option<u16>,

        #[arg(long, short, default_value_t = false)]
        verbose: bool,
    },

    /// Start a server
    Serve {
        bind_host: String,

        #[arg(long, short, value_parser = port_in_range)]
        port: u16,

        #[arg(long, default_value_t = false)]
        tls: bool,

        #[arg(long, value_parser = valid_path)]
        cert: Option<String>,

        #[arg(long, value_parser = valid_path)]
        key: Option<String>,

        #[arg(long, default_value_t = false)]
        udp: bool,

        #[arg(short, long)]
        exec: Option<String>,

        #[arg(long, short, default_value_t = false)]
        verbose: bool,
    },
}

const PORT_RANGE: RangeInclusive<usize> = 1..=65535;

fn port_in_range(s: &str) -> Result<u16, String> {
    let port: usize = s
        .parse()
        .map_err(|_| format!("`{}` is not a valid port number", s))?;
    if PORT_RANGE.contains(&port) {
        Ok(port as u16)
    } else {
        Err(format!(
            "Port not in range {}-{}",
            PORT_RANGE.start(),
            PORT_RANGE.end()
        ))
    }
}

fn valid_path(s: &str) -> Result<String, String> {
    let path = Path::new(s);

    if path.exists() {
        Ok(s.to_string())
    } else {
        Err(format!(
            "Path does not exist {}",
            s,
        ))
    }
}
