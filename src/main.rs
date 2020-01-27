use std::io;
use std::thread;
use std::sync::{Arc, Mutex};
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::collections::VecDeque;

const SERVER_NAME: &'static str = "rpush/0";
const HTTP_VER: &'static str = "HTTP/1.1";
const READ_BUF_LEN: usize = 4096;
const MAX_THREAD_COUNT: usize = 4;


fn handle_client(mut stream: TcpStream) {
    // println!("addr: {:?}", stream.peer_addr());

    let mut ibuf = [0; READ_BUF_LEN];
    stream.read(&mut ibuf).expect("Failed to read");
    let istr = String::from_utf8(ibuf.iter().take_while(|&&x| x != 0).cloned().collect()).unwrap();
    // println!("{}", istr);

    let obody = format!("<html>\n<head><title>rpush</title></head>\n<body>\n<center><h1>Hello web from rust!</h1><d>{}</d></center>\n<hr><center>rpush/0</center>\n</body>\n</html>\n", istr);
    let ostr = format!("{http} 200 OK\nServer: {srv}\nContent-Type:{type}\nContent-Length: {len}\n\n{body}", http=HTTP_VER, srv=SERVER_NAME, type="text/html", len=obody.len(), body=obody);
    let obuf = ostr.as_bytes();

    stream.write(&obuf).unwrap();

    stream.shutdown(std::net::Shutdown::Both).expect("Failed to shutdown!");
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("192.168.1.69:80")?;
    listener.set_ttl(100).expect("Can't set TTL");
    listener.set_nonblocking(true).expect("Can't set nonblocking!");

    let mut queue: VecDeque<TcpStream> = VecDeque::new();
    let thread_count = Arc::new(Mutex::new(0));

    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                queue.push_back(stream);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                let _thread_count = thread_count.clone();
                if *thread_count.lock().unwrap() < MAX_THREAD_COUNT {
                    match queue.pop_front() {
                        Some(stream) => {
                            thread::spawn(move || {
                                *_thread_count.lock().unwrap() += 1;
                                handle_client(stream);
                                *_thread_count.lock().unwrap() -= 1;
                            });
                        },
                        None => {},
                    };
                }
                
                continue;
            }
            Err(e) => { panic!("{:?}", e); }
        }
    }

    Ok(())
}
