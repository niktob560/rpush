use std::io;
use std::thread;
use std::sync::{Arc, Mutex};
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::collections::VecDeque;

extern crate config;


use std::collections::HashMap;

const SERVER_NAME: &'static str = "rpush/0";
const HTTP_VER: &'static str = "HTTP/1.1";


fn handle_client(mut stream: TcpStream, buf_len: usize) {
    let mut ibuf = Vec::<u8>::new();
    ibuf.resize(buf_len, 0);
    stream.read(&mut ibuf).expect("Read error!");
    // ibuf.iter().take_while()
    // let t = ibuf.iter().take_while(|&&x| x != 0).cloned().collect::<Vec<u8>>();
    let istr = match String::from_utf8(ibuf.iter().take_while(|&&x| x != 0).cloned().collect::<Vec<u8>>()) {
        Ok(x) => x,
        Err(_) => "".to_string(),
    };

    let obody = format!("<html>
<head><title>rpush</title></head>
<body>
<center><h1>Hello web from rust!</h1>
<d>
{}
</d>
</center>
<hr><center>rpush/0</center>
</body>
</html>
", istr);
    let ostr = format!("{http} 200 OK
Server: {srv}
Content-Type:{type}
Content-Length: {len}

{body}", http=HTTP_VER, srv=SERVER_NAME, type="text/html", len=obody.len(), body=obody);
    let obuf = ostr.as_bytes();

    stream.write(&obuf).unwrap();

    stream.shutdown(std::net::Shutdown::Both).expect("Failed to shutdown!");
}

fn main() -> io::Result<()> {

    let mut settings_bundle: config::Config = config::Config::default();
    settings_bundle
        .merge(config::File::with_name("rpush_config")).unwrap()
        .merge(config::Environment::with_prefix("APP")).unwrap();

    let settings: HashMap<String, String> = 
        match settings_bundle.try_into::<HashMap<String, String>>() {
            Ok(x) => x,
            Err(_) => HashMap::new(),
    };

    let ip = match settings.get("ip") {
        Some(x) => x.clone(),
        None => "0.0.0.0".to_string(),
    };

    let port = match settings.get("port") {
        Some(x) => x.clone(),
        None => "80".to_string(),
    };

    let max_threads = match settings.get("threads") {
        Some(x) => x.parse::<u32>().unwrap(),
        None => 4,
    };

    let max_read_len = match settings.get("max_read_len") {
        Some(x) => x.parse::<usize>().unwrap(),
        None => 4096,
    };
    
    println!("Max packet read len set on {}", max_read_len);
    println!("Thread lock set on {}", max_threads);
    println!("Binding on {}:{}", ip, port);

    let listener = TcpListener::bind(format!("{}:{}", ip, port))?;
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
                if *thread_count.lock().unwrap() < max_threads {
                    match queue.pop_front() {
                        Some(stream) => {
                            thread::spawn(move || {
                                *_thread_count.lock().unwrap() += 1;
                                handle_client(stream, max_read_len);
                                *_thread_count.lock().unwrap() -= 1;
                            });
                        },
                        None => {},
                    };
                }
                else {
                    thread::sleep(std::time::Duration::new(0, 1000));
                }
                continue;
            }
            Err(e) => { panic!("{:?}", e); }
        }
    }

    Ok(())
}
