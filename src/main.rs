use std::io::prelude::*;
use std::collections::HashMap;
use std::time::Instant;
use std::hash::{Hash, Hasher};
use std::net::TcpStream;
use std::net::TcpListener;

use clap::clap_app;

#[derive(Debug)]
struct MemDB {
    data: HashMap<MemKey, MemVal>,
    _init_time: Instant
}

impl MemDB {
    fn new() -> MemDB {
        MemDB {
            data: HashMap::<MemKey, MemVal>::new(),
            _init_time: Instant::now()
        }
    }
    fn store(&mut self, key: String, val: String) {
        self.data.insert(MemKey::new(key), MemVal::new(val));
    }
    // key input should be reference, ineffecient right now
    fn fetch(&self, key: String) -> Option<&String> {
        let res = self.data.get(&MemKey::new(key));
        if let Some(val) = res {
            return Some(&val.value);
        }
        None
    }
}

#[derive(Debug)]
struct MemVal {
    pub value: String,
    _last_modified: Instant,
}

impl MemVal {
    pub fn new (value: String) -> MemVal {
        MemVal {
            value,
            _last_modified: Instant::now()
        }
    }
}

#[derive(Debug)]
struct MemKey {
    value: String,
    _created: Instant
}

impl PartialEq for MemKey {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for MemKey {} 

impl Hash for MemKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

impl MemKey {
    pub fn new (value: String) -> MemKey {
        MemKey {
            value: value,
            _created: Instant::now()
        }
    }
}


struct MemDaemon {
    listener: TcpListener
}

impl MemDaemon {
    pub fn new(host: String, port: String) -> MemDaemon {
        let addr = format!("{}:{}", host, port);
        println!("Starting daemon at {}", addr);
        let listener = TcpListener::bind(addr).expect("Failed to start daemon");
        MemDaemon {
            listener
        }
    }
    
    fn run(&self) {
        println!("Starting the MemDB server");
        for stream in self.listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buffer = [0; 512];
            stream.read(&mut buffer).unwrap();
            let buffer: String = String::from_utf8_lossy(&buffer).to_string();
            self.handle_stream(&buffer);
            stream.write(&[1]);
        }
    }

    fn handle_stream(&self, buffer: &str) {
        println!("Recvd is  {:?}", buffer);
    }
}

struct MemClient {
    stream: TcpStream
}

impl MemClient {
    pub fn connect(host: String, port: String) -> MemClient {
        MemClient {
            stream: TcpStream::connect(format!("{}:{}", host, port)).expect("Failed to connec to the server") 
        }
    }

    pub fn send(&mut self, msg: String) {
        let mut buffer = [0; 512];
        self.stream.write(&[1]);
        self.stream.read(&mut buffer).unwrap();
        let buffer = String::from_utf8_lossy(&buffer).to_string();
        println!("Recv from server {:?}", buffer);
    }
}

fn main() {
    let matches = clap_app!(myapp =>
        (version: "1.0.0")
        (author: "clearnote01")
        (about: "CLI to start memd datastore or fetch/store from it")
        (@arg host: -h --host +takes_value 
            default_value("127.0.0.1") "hostname for the tcp server")
        (@arg port: -p --port +takes_value 
            default_value("7000") "port number for the tcp server")
        (@subcommand fetch =>
                        (about: "fetch val for a key")
                        (@arg key: -k --key +required +takes_value "Key that was previously stored")
        )
        (@subcommand store =>
                        (about: "store key:val pair")
                        (@arg key: +required "key name")
                        (@arg val: +required "value name")
        )
        (@subcommand daemon =>
                        (about: "Run as the daemon")
        )
    ).get_matches();


    let mut mem = MemDB::new();
    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();

    if let Some(_) = matches.subcommand_matches("daemon") {
        let mem_daemon = MemDaemon::new(host.to_string(), port.to_string());
        mem_daemon.run();
    }

    let mut mem_client = MemClient::connect(host.to_string(), port.to_string());
    

    if let Some(matches) = matches.subcommand_matches("fetch") {
        let key = matches.value_of("key").unwrap();
        let val = mem.fetch(key.to_string());
        println!("value is {:?}", val);
        mem_client.send(key.to_string());
    }

    if let Some(matches) = matches.subcommand_matches("store") {
        let key = matches.value_of("key").unwrap();
        let val = matches.value_of("val").unwrap();
        mem.store(key.to_string(), val.to_string());
        mem_client.send(key.to_string());
    }

    println!("{:?}", mem);
}
