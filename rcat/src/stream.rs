use std::io::Error;
use crate::common::{read_write, read_write_exec};

pub async fn connect(host: &String, port: &u16) -> Result<(), Error> {
    let addr = format!("{}:{}", host, port);
    let client = tokio::net::TcpStream::connect(addr.as_str()).await?;

    let (reader, writer) = client.into_split();

    read_write(reader, writer).await;

    Ok(())
}

pub async fn serve(bind_host: &String, port: &u16) -> Result<(), Error> {
    let addr = format!("{}:{}", bind_host, port);
    let listener = tokio::net::TcpListener::bind(addr.as_str()).await?;

    let (handle, _) = listener.accept().await?;

    let (reader, writer) = handle.into_split();

    read_write(reader, writer).await;

    Ok(())
}

pub async fn serve_exec(bind_host: &String, port: &u16, exec: String) -> Result<(), Error> {
    let addr = format!("{}:{}", bind_host, port);
    let listener = tokio::net::TcpListener::bind(addr.as_str()).await?;

    let (handle, _) = listener.accept().await?;

    let (reader, writer) = handle.into_split();

    read_write_exec(reader, writer, exec).await;

    Ok(())
}
