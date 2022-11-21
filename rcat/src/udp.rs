use tokio::io::{stdin, stdout};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::select;
use std::io::Error;

pub async fn udp_connect(host: &String, port: &u16, listen_port: u16) -> Result<(), Error> {
    let addr = format!("0.0.0.0:{}", listen_port);
    let socket = UdpSocket::bind(addr).await?;

    let remote_addr = format!("{}:{}", host, port);
    socket.connect(remote_addr).await?;

    let mut stdin_buf = [0; 512];
    let mut network_in_buf = [0; 512];

    let mut stdin = stdin();

    let mut active = true;

    while active {
        select! {
            res = stdin.read(&mut stdin_buf) => {
                if let Ok(amount) = res {
                    if amount != 0 {
                        socket.send(&stdin_buf[0..amount]).await.map_err(|_| "failed to write to the socket").unwrap();
                    } else {
                        active = false;
                    }
                } else {
                    res.unwrap();
                }
            }
            res = socket.recv(&mut network_in_buf) => {
                if let Ok(amount) = res {
                    // Note there is no associated else because a zero length UDP message is valid. Just skip writing
                    if amount != 0 {
                        stdout().write(&network_in_buf[0..amount]).await.map_err(|_| "failed to write to stdout").unwrap();
                    }
                } else {
                    res.unwrap();
                }
            }
        }
    }

    Ok(())
}

pub async fn udp_serve(bind_host: &String, port: &u16) -> Result<(), Error> {
    let addr = format!("{}:{}", bind_host, port);
    let socket = UdpSocket::bind(addr).await?;

    let mut stdin_buf = [0; 512];
    let mut network_in_buf = [0; 512];

    let mut stdin = stdin();

    let mut is_connected = false;
    let mut stdin_tmp_buf = Vec::new();

    let mut active = true;

    while active {
        select!(
            res = socket.recv_from(&mut network_in_buf) => {
                if let Ok((amount, remote_addr)) = res {
                    // Note there is no associated else because a zero length UDP message is valid. Just skip writing
                    if amount != 0 {
                        stdout().write(&network_in_buf[0..amount]).await.map_err(|_| "failed to write to stdout").unwrap();
                    }

                    if !is_connected {
                        socket.connect(remote_addr).await.unwrap();
                        is_connected = true;

                        // Output pending data from stdin
                        if stdin_tmp_buf.len() != 0 {
                            socket.send(stdin_tmp_buf.as_ref()).await.map_err(|_| "failed to write to the socket").unwrap();
                            stdin_tmp_buf.clear();
                        }
                    }
                } else {
                    res.unwrap();
                }
            }
            res = stdin.read(&mut stdin_buf) => {
                if let Ok(amount) = res {
                    if amount != 0 {
                        if is_connected {
                            socket.send(&stdin_buf[0..amount]).await.map_err(|_| "failed to write to the socket").unwrap();
                        } else {
                            stdin_tmp_buf.extend_from_slice(&stdin_buf[0..amount]);
                        }
                    } else {
                        active = false;
                    }
                } else {
                    res.unwrap();
                }
            }
        )
    }

    Ok(())
}
