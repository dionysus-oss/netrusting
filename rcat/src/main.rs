use std::io::{stdout, Write};
use std::time::Duration;
use clap::Parser;
use cli::{Cli, Commands};
use futures::FutureExt;

use crate::stream::{connect, serve, serve_exec};
use crate::tls::{tls_connect, tls_serve};
use crate::udp::{udp_connect, udp_serve};

mod cli;
mod stream;
mod tls;
mod common;
mod udp;

fn main() {
    let cli = Cli::parse();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    match &cli.command {
        Commands::Connect { host, port, tls, ca, udp, listen_port, verbose } => {
            if *verbose {
                println!("connecting to {}:{}", host, port);
            }

            let connect_future = if *tls {
                tls_connect(host, port, ca).boxed()
            } else if *udp {
                udp_connect(host, port, listen_port.clone().expect("listen-port is required")).boxed()
            } else {
                connect(host, port).boxed()
            };

            runtime.block_on(async {
                tokio::select! {
                    res = connect_future => {
                        if let Err(e) = res {
                            eprintln!("connect failed: {}", e.to_string());
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {}
                }
            });
        }
        Commands::Serve { bind_host, port, tls, cert, key, udp, exec, verbose } => {
            if *verbose {
                println!("bind to {}:{}", bind_host, port);
            }

            let serve_future = if *tls {
                tls_serve(bind_host, port, cert.clone().expect("cert is required"), key.clone().expect("key is required")).boxed()
            } else if *udp {
                udp_serve(bind_host, port).boxed()
            } else if let Some(ref exec) = exec {
                serve_exec(bind_host, port, exec.clone()).boxed()
            } else {
                serve(bind_host, port).boxed()
            };

            runtime.block_on(async {
                tokio::select! {
                    res = serve_future => {
                        if let Err(e) = res {
                            eprintln!("listen failed: {}", e.to_string());
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {}
                }
            });
        }
    }

    stdout().flush().expect("failed to flush stdout");

    // https://github.com/tokio-rs/tokio/issues/2318
    runtime.shutdown_timeout(Duration::from_secs(0));
}
