use std::io::Write;
use std::ops::RangeInclusive;
use std::process;
use std::time::Duration;
use clap::{Parser, Subcommand};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::runtime::Handle;

mod connect;
mod serve;
mod stream;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Connect to a server
    Connect {
        host: String,

        #[arg(short, long, value_parser = port_in_range)]
        port: u16,
    },

    /// Start a server
    Serve {
        #[arg(default_value = "127.0.0.1")]
        bind_host: String,

        #[arg(short, long, value_parser = port_in_range)]
        port: u16,
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

fn main() {
    let cli = Cli::parse();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    match &cli.command {
        Commands::Connect { host, port } => {
            println!("connect to {}:{}", host, port);

            runtime.block_on(async {
                tokio::select!(
                    _ = stream::client(&runtime) => {}
                    _ = tokio::signal::ctrl_c() => {}
                );
            });
        }
        Commands::Serve { bind_host, port } => {
            println!("bind to {}:{}", bind_host, port);

            runtime.block_on( async {
                tokio::select!(
                    val = stream::server(&runtime) => {}
                    _ = tokio::signal::ctrl_c() => {}
                );
            });
        }
    }

    runtime.shutdown_timeout(Duration::from_secs(0));
}
