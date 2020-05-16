use std::io::prelude::*;
use serde::{Serialize, Deserialize};
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum ReqMemMsg { // different type of tcp request type that can be made
    StoreKeyVal(String, String),
    FetchKey(String),
}

impl ReqMemMsg {
    fn get_byte_arr(&self) -> Vec<u8> {
        // figuring the below part took forever
        // what i learned was slice can be made from vector also, same as array
        // array cannot be dynamically allocated
          
        let send_data = bincode::serialize(&self).unwrap();

        let size_send_data: u16 = send_data.len() as u16;
        let size_arr = size_send_data.to_be_bytes(); // converting to bigendian order arr

        let byte_arr: Vec<u8> = size_arr.iter().chain(send_data.iter()).map(|x| *x).collect();
        
        byte_arr
    }
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
    listener: TcpListener,
    mem: MemDB
}

impl MemDaemon {
    pub fn new(host: String, port: String) -> MemDaemon {
        let addr = format!("{}:{}", host, port);
        println!("Starting daemon at {}", addr);
        let listener = TcpListener::bind(addr).expect("Failed to start daemon");
        MemDaemon {
            listener,
            mem: MemDB::new()
        }
    }
    
    fn run(&mut self) {
        println!("Starting the MemDB server");
        for stream in self.listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buffer = [0; 512];
            stream.read(&mut buffer).unwrap();

            // first two digits store the length of the objects
            let len: u16 = ((buffer[0] as u16) << 8) | (buffer[1] as u16);
            // taking the slice which contains encoded object
            let byte_obj = &buffer[2..(2 + len as usize)];
            
            let resp: String;
            match bincode::deserialize(&byte_obj) {
                Ok(decoded) => {
                    match decoded {
                        ReqMemMsg::StoreKeyVal(key, val) => {
                            self.mem.store(key.to_string(), val.to_string());
                            resp = "Key:Val saved in mem".to_string();
                        },
                        ReqMemMsg::FetchKey(key) => {
                            let val = self.mem.fetch(key.to_string());
                            match val {
                                Some(key_value) => resp = key_value.to_string(),
                                None => resp = "key not found".to_string()
                            }
                        }
                    };
                    println!("Mem now {:?}", self.mem);
                },
                _ => {
                    resp = "not of acceptable request message format".to_string();
                }
            }
            stream.write(resp.as_bytes()).unwrap();
        }
    }

    fn _handle_utf8(&self, buffer: &[u8]) {
        let buffer: String = String::from_utf8_lossy(&buffer).to_string();
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

    pub fn send(&mut self, msg: &[u8]) {
        // let msg: &[u8] = msg.as_bytes();
        let mut buffer = [0; 512];
        self.stream.write(msg).unwrap();
        self.stream.read(&mut buffer).unwrap();
        let buffer = String::from_utf8_lossy(&buffer).to_string();
        println!("Recv from server {:?}", buffer);
    }
}

fn main() {
    let matches = clap_app!("" =>
        (about: "CLI to start memd datastore or fetch/store from it")
        (@arg host: -h --host +takes_value 
            default_value("127.0.0.1") "hostname for the tcp server")
        (@arg port: -p --port +takes_value 
            default_value("7000") "port number for the tcp server")
        (@subcommand fetch =>
                        (about: "fetch val for a key")
                        (@arg key: +required +takes_value "Key that was previously stored")
        )
        (@subcommand store =>
                        (about: "store key:val pair")
                        (@arg key: +required "key")
                        (@arg val: +required "value")
        )
        (@subcommand daemon =>
                        (about: "Run as the daemon")
        )
    ).get_matches();

    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();

    if let Some(_) = matches.subcommand_matches("daemon") {
        let mut mem_daemon = MemDaemon::new(host.to_string(), port.to_string());
        mem_daemon.run();
    }

    let mut mem_client = MemClient::connect(host.to_string(), port.to_string());
    

    if let Some(matches) = matches.subcommand_matches("fetch") {
        let key = matches.value_of("key").unwrap();

        let msg = ReqMemMsg::FetchKey(key.to_string());
        let msg_byte_arr = msg.get_byte_arr();
        mem_client.send(&msg_byte_arr[..]);
    }

    if let Some(matches) = matches.subcommand_matches("store") {
        let key = matches.value_of("key").unwrap();
        let val = matches.value_of("val").unwrap();

        let msg = ReqMemMsg::StoreKeyVal(key.to_string(),  val.to_string());
        let msg_byte_arr = msg.get_byte_arr();
        mem_client.send(&msg_byte_arr[..]);
    }
}
