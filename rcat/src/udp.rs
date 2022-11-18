use std::io;
use tokio::io::{stdout, stdin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::mpsc;
use crate::common::read_write;

pub async fn udp_connect(host: &String, port: &u16, listen_port: u16) -> Result<(), String> {
    let addr = format!("0.0.0.0:{}", listen_port);
    let socket = UdpSocket::bind(addr).await.map_err(|_| "failed to bind return port")?;

    let remote_addr = format!("{}:{}", host, port);
    socket.connect(remote_addr).await.unwrap();

    let mut stdin_buf = [0; 512];
    let mut network_in_buf = [0; 512];

    let mut stdin = stdin();

    let mut active = true;

    while active {
        select! {
            res = stdin.read(&mut stdin_buf) => {
                if let Ok(amt) = res {
                    if amt != 0 {
                        socket.send(&stdin_buf[0..amt]).await.map_err(|_| "failed to write to stdout").unwrap();
                    }
                } else {
                    res.unwrap();
                }
            }
            res = socket.recv(&mut network_in_buf) => {
                 if let Ok(amt) = res {
                    if amt != 0 {
                        stdout().write(&network_in_buf[0..amt]).await.map_err(|_| "failed to write to stdout").unwrap();
                    }
                } else {
                    res.unwrap();
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("received terminate signal");
                active = false;
            }
        }
    }

    println!("UDP connect finished");

    Ok(())
}

pub async fn udp_serve(bind_host: &String, port: &u16) -> Result<(), String> {
    let addr = format!("{}:{}", bind_host, port);
    let socket = UdpSocket::bind(addr).await.map_err(|_| "failed to bind")?;

    let mut stdin_buf = [0; 512];
    let mut network_in_buf = [0; 512];

    let mut stdin = stdin();

    let mut is_connected = false;
    let mut stdin_tmp_buf = Vec::new();

    let mut active = true;

    while active {
        select! {
            res = socket.recv_from(&mut network_in_buf) => {
                 if let Ok((amt, remote_addr)) = res {
                    if amt != 0 {
                        // if is_connected {
                        //     // Reject connections from other clients
                        // }

                        stdout().write(&network_in_buf[0..amt]).await.map_err(|_| "failed to write to stdout").unwrap();

                        if !is_connected {
                            socket.connect(remote_addr).await.unwrap();
                            is_connected = true;
                        }
                    }
                } else {
                    res.unwrap();
                }
            }
            res = stdin.read(&mut stdin_buf) => {
                if let Ok(amt) = res {
                    if amt != 0 {
                        if is_connected {
                            if stdin_tmp_buf.len() != 0 {
                                socket.send(stdin_tmp_buf.as_ref()).await.map_err(|_| "failed to write to stdout").unwrap();
                                stdin_tmp_buf.clear();
                            }

                            socket.send(&stdin_buf[0..amt]).await.map_err(|_| "failed to write to stdout").unwrap();
                        } else {
                            stdin_tmp_buf.extend_from_slice(&stdin_buf[0..amt]);
                        }
                    }
                } else {
                    res.unwrap();
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("received terminate signal");
                active = false
            }
        }
    }

    println!("UDP serve finished");

    Ok(())
}
