use std::io::{stdout, Write};
use std::ops::RangeInclusive;
use std::path::Path;
use std::time::Duration;
use clap::{Parser, Subcommand};

mod connect;
mod serve;
mod stream;
mod tls;
mod common;
mod udp;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Connect to a server
    Connect {
        host: String,

        #[arg(short, long, value_parser = port_in_range)]
        port: u16,

        #[arg(long, value_parser = port_in_range)]
        listen_port: Option<u16>,

        #[arg(long, value_parser = valid_path)]
        ca: Option<String>
    },

    /// Start a server
    Serve {
        #[arg(default_value = "127.0.0.1")]
        bind_host: String,

        #[arg(short, long, value_parser = port_in_range)]
        port: u16,

        #[arg(long, value_parser = valid_path)]
        ca: Option<String>,

        #[arg(long, value_parser = valid_path)]
        cert: Option<String>,

        #[arg(long, value_parser = valid_path)]
        key: Option<String>,

        #[arg(short, long)]
        exec: Option<String>,
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

fn main() {
    let cli = Cli::parse();

    //let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
    //addr.is_ipv4();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    match &cli.command {
        Commands::Connect { host, port, listen_port, ca } => {
            println!("connect to {}:{}", host, port);

            runtime.block_on(async {
                tokio::select! {
                    // res = udp::udp_connect(host, port, listen_port.clone().expect("missing listen port")) => {
                    //     if let Err(e) = res {
                    //         println!("connect failed: {}", e.to_string());
                    //     }
                    // }

                    // res = tls::tls_connect(host, port, ca) => {
                    //     if let Err(e) = res {
                    //         println!("connect failed: {}", e.to_string());
                    //     }
                    // }

                    _ = stream::client(host, port) => {}
                    _ = tokio::signal::ctrl_c() => {}
                }
            });
        }
        Commands::Serve { bind_host, port, ca, cert, key, exec } => {
            println!("bind to {}:{}", bind_host, port);

            runtime.block_on(async {
                tokio::select! {
                    // res = udp::udp_serve(bind_host, port) => {
                    //     if let Err(e) = res {
                    //         println!("serve failed: {}", e.to_string());
                    //     }
                    // }

                    // res = tls::tls_listen(bind_host, port, ca, cert.clone().expect("cert is required"), key.clone().expect("cert is required")) => {
                    //     if let Err(e) = res {
                    //         println!("listen failed: {}", e.to_string());
                    //     }
                    // }
                    _ = stream::serve_exec(bind_host, port, exec.clone().expect("exec is required")) => {}
                    _ = tokio::signal::ctrl_c() => {}
                }
            });
        }
    }

    stdout().flush().expect("failed to flush stdout");

    // https://github.com/tokio-rs/tokio/issues/2318
    runtime.shutdown_timeout(Duration::from_secs(0));
}
