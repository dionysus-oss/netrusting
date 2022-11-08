use crate::common::read_write;

pub async fn client(host: &String, port: &u16) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);
    let client = tokio::net::TcpStream::connect(addr.as_str()).await.map_err(|_| "failed to connect")?;

    let (mut reader, mut writer) = client.into_split();

    let client_read = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });

    let client_write = tokio::spawn(async move {
        tokio::io::copy(&mut tokio::io::stdin(), &mut writer).await
    });

    tokio::select! {
        _ = client_read => {}
        _ = client_write => {}
    }

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
