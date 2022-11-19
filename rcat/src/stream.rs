use crate::common::{read_write, read_write_exec};

pub async fn client(host: &String, port: &u16) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);
    let client = tokio::net::TcpStream::connect(addr.as_str()).await.map_err(|_| "failed to connect")?;

    let (mut reader, mut writer) = client.into_split();

    read_write(reader, writer).await;

    Ok(())
}

pub async fn server(bind_host: &String, port: &u16) -> Result<(), String> {
    let addr = format!("{}:{}", bind_host, port);
    let listener = tokio::net::TcpListener::bind(addr.as_str()).await.map_err(|_| "failed to bind")?;

    let (handle, _) = listener.accept().await.map_err(|_| "failed to accept")?;

    let (reader, writer) = handle.into_split();

    read_write(reader, writer).await;

    Ok(())
}

pub async fn serve_exec(bind_host: &String, port: &u16, exec: String) -> Result<(), String> {
    let addr = format!("{}:{}", bind_host, port);
    let listener = tokio::net::TcpListener::bind(addr.as_str()).await.map_err(|_| "failed to bind")?;

    let (handle, _) = listener.accept().await.map_err(|_| "failed to accept")?;

    let (reader, writer) = handle.into_split();

    read_write_exec(reader, writer, exec).await;

    Ok(())
}
