use std::io;
use std::thread;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::collections::VecDeque;

extern crate config;


use std::collections::HashMap;

const SERVER_NAME: &'static str = "rpush/0";
const HTTP_VER: &'static str = "HTTP/1.1";


const ERR_BODY_TEMPLATE: &'static str = "<html>
<head><title>{ERR}</title></head>
<body>
    <center><h1>{ERR}</h1></center>
    <hr><center>{SRV_NAME}</center>
</body>
</html>";


fn get_err_body(kind: io::ErrorKind) -> (String, usize, u16) {
    let code: u16;
    let body =  ERR_BODY_TEMPLATE.to_string()
                .replace("{SRV_NAME}", SERVER_NAME)
                .replace("{ERR}",
                    match kind {
                        io::ErrorKind::NotFound => {
                            code = 404;
                            "File not found 404"
                        },
                        io::ErrorKind::PermissionDenied => {
                            code = 403;
                            "Permission denied 403"
                        }
                        _ => {
                            code = 500;
                            println!("err {:?}", kind);
                            "Internal server error 500"
                        },
                    });
    let len = body.len();
    (body, len, code)
}


fn gener_body_get(file: &str, ops: &str, site_dir: &str) -> Result<(String, usize), io::Error> {
    let mut file_bundle = match File::open(format!("{}{}", site_dir, file)) {
        Ok(f) => f,
        Err(e) => {
            return Err(e);
        },
    };

    let mut obody = String::new();
    let size = match file_bundle.read_to_string(&mut obody) {
        Ok(u) => u,
        Err(e) => {
            return Err(e);
        },
    };
    Ok((obody, size))
}


fn handle_client(mut stream: TcpStream, buf_len: usize, site_dir: &str) {
    //get ingoing packet
    let mut ibuf = Vec::<u8>::new();
    ibuf.resize(buf_len, 0);
    stream.read(&mut ibuf).expect("Read error!");

    //get ingoing packet as string
    let istr: String = match String::from_utf8(ibuf.iter().take_while(|&&x| x != 0).cloned().collect::<Vec<u8>>()) {
        Ok(x) => x,
        Err(_) => "".to_string(),
    };

    //get peer header
    let header: Vec<&str> = istr.split('\n').collect();

    //get peer raw query
    let query_raw = match header.iter().find(|s| String::from(**s).contains("HTTP/")) {
        Some(x) => String::from(*x),
        None => "".to_string(),
    };

    //parse query {TYPE} /file?params HTTP/{VER} to /file?params
    let query: String = query_raw.split(' ').skip(1).take(1).collect();

    //get file from query
    let file: String = query.split('?').take(1).collect();

    //get params from query
    let ops: String = query.split('?').skip(1).take(1).collect();

    //get type of connection
    let _query_type_str: String = query_raw.split(' ').take(1).collect::<String>();

    //get body of response and it's len
    let (obody, len, code): (String, usize, u16) = match _query_type_str.as_str() {
        "GET" =>
            match gener_body_get(file.as_str(), ops.as_str(), site_dir) {
                Ok(x) => (x.0, x.1, 200),
                Err(e) => {
                    println!("GET error: {:?}", e);
                    get_err_body(e.kind())
                },
            }
        ,
        _ => {
            println!("Bad conn type: {}", _query_type_str);
            get_err_body(io::ErrorKind::Other)
        },
    };

    println!("code {code} at {type} query of {addr}: {query}", code=code, type=_query_type_str, addr=format!("{:?}", stream.peer_addr()), query=query);

    //format header and body
    let ostr = format!("{http} {code}\nServer: {srv}\nContent-Type:{type}\nContent-Length: {len}\n\n{body}"
                , http=HTTP_VER, code=code, srv=SERVER_NAME, type="text/html",           len=len, body=obody);

    //convert output string with header to bytes
    let obuf = ostr.as_bytes();

    //send btes to peer
    stream.write(&obuf).unwrap();

    //close connection
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

    let site_dir = match settings.get("site_dir") {
        Some(x) => Arc::new(Mutex::new(x.clone())),
        None => Arc::new(Mutex::new(".".to_string())),
    };

    let await_ns = match settings.get("await_ns") {
        Some(x) => x.parse::<u32>().unwrap(),
        None => 10000,
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
                if *thread_count.lock().unwrap() < max_threads {
                    match queue.pop_front() {
                        Some(stream) => {
                            let _thread_count = thread_count.clone();
                            let _site_dir_arc = site_dir.clone();
                            thread::spawn(move || {
                                *_thread_count.lock().unwrap() += 1;
                                handle_client(stream, max_read_len, &*_site_dir_arc.lock().unwrap().as_str());
                                *_thread_count.lock().unwrap() -= 1;
                            });
                        },
                        None => {},
                    };
                }
                else {
                    if queue.len() < 30 {
                        thread::sleep(std::time::Duration::new(0, await_ns));
                    }
                }
                continue;
            }
            Err(e) => { panic!("{:?}", e); }
        }
    }

    Ok(())
}
