use std::io::Write;
use std::net::TcpStream;

pub fn run(host: &String, port: &u16) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);
    let mut client = TcpStream::connect(addr.clone()).map_err(|_| format!("failed to connect to {}", addr))?;

    client.write("hello, TCP".as_bytes()).map_err(|_| format!("failed to send"))?;

    Ok(())
}
