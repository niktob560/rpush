use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};


fn handle_client(mut stream: TcpStream) {
    // println!(stream.);
    // stream.write("buf: &[u8]")
    // stream.write(&[1]);
    println!("addr: {:?}", stream.peer_addr());
    let obuf = "Hello world from rust!\n\r".as_bytes();
    stream.write(&obuf).unwrap();
}

fn main() -> std::io::Result<()> {
    let addrs = [
        SocketAddr::from(([192, 168, 1, 69], 80)),
        SocketAddr::from(([127, 0, 0, 1], 80)),
        SocketAddr::from(([0, 0, 0, 0], 80)),
    ];

    let listener = TcpListener::bind(&addrs[..])?;
    listener.set_ttl(100).expect("Can't set TTL");
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new client!");
                handle_client(stream);
            }
            Err(e) => { panic!("{:?}", e); }
        }
    }

    Ok(())
}
