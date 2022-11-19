use std::io::{Read, Stdout, Write};
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use tokio::select;

pub async fn read_write<R, W>(mut reader: R, mut writer: W) where
    R: AsyncRead + Unpin + Sized + Send + 'static,
    W: AsyncWrite + Unpin + Sized + Send + 'static {
    let client_read = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });

    let client_write = tokio::spawn(async move {
        tokio::io::copy(&mut tokio::io::stdin(), &mut writer).await
    });

    select! {
        _ = client_read => {}
        _ = client_write => {}
    }
}

pub async fn read_write_exec<R, W>(mut reader: R, mut writer: W, exec: String) where
    R: AsyncRead + Unpin + Sized + Send + 'static,
    W: AsyncWrite + Unpin + Sized + Send + 'static {
    let mut child = tokio::process::Command::new(exec)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().unwrap();

    let mut stdin = child.stdin.unwrap();
    let mut stdout = child.stdout.unwrap();
    let mut stderr = child.stderr.unwrap();

    let mut active = true;

    let mut stdout_buf = [0; 512];
    let mut stderr_buf = [0; 512];
    let mut network_in_buf = [0; 512];

    while active {
        select! {
            res = stdout.read(&mut stdout_buf) => {
                if let Ok(amount) = res {
                    if amount != 0 {
                        writer.write(&stdout_buf[0..amount]).await.map_err(|_| "failed to write to the socket").unwrap();
                    } else {
                        active = false;
                    }
                } else {
                    res.unwrap();
                }
            }
            res = stderr.read(&mut stderr_buf) => {
                if let Ok(amount) = res {
                    if amount != 0 {
                        writer.write(&stderr_buf[0..amount]).await.map_err(|_| "failed to write to the socket").unwrap();
                    } else {
                        active = false;
                    }
                } else {
                    res.unwrap();
                }
            }
            res = reader.read(&mut network_in_buf) => {
                if let Ok(amount) = res {
                    if amount != 0 {
                        stdin.write(&network_in_buf[0..amount]).await.map_err(|_| "failed to write to stdout").unwrap();
                    } else {
                        active = false;
                    }
                } else {
                    res.unwrap();
                }
            }
        }
    }
}
