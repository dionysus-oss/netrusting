use std::borrow::Borrow;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{ReadHalf, WriteHalf};
use tokio::task::JoinHandle;

pub async fn client(runtime: &tokio::runtime::Runtime) -> Result<(), String> {
    let mut client = tokio::net::TcpStream::connect("localhost:2323").await.map_err(|_| "failed to connect")?;

    let (mut reader, mut writer) = client.into_split();

    let server_read = runtime.spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });

    let server_write = runtime.spawn(async move {
        tokio::io::copy(&mut tokio::io::stdin(), &mut writer).await
    });

    tokio::select!(
        _ = server_read => {}
        _ = server_write => {}
        _ = tokio::signal::ctrl_c() => {}
    );

    return Ok(());
}

pub async fn server(runtime: &tokio::runtime::Runtime) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("localhost:2323").await.map_err(|_| "failed to bind server")?;

    let (handle, _) = listener.accept().await.map_err(|_| "failed to accept")?;

    let (mut reader, mut writer) = handle.into_split();

    let server_read = runtime.spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });

    let server_write = runtime.spawn(async move {
        tokio::io::copy(&mut tokio::io::stdin(), &mut writer).await
    });

    tokio::select!(
        _ = server_read => {}
        _ = server_write => {}
        _ = tokio::signal::ctrl_c() => {}
    );

    return Ok(())
}
