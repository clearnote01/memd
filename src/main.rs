use std::io::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::Instant;
use std::hash::{Hash, Hasher};
use std::net::TcpStream;
use std::net::TcpListener;

use clap::clap_app;

trait MemMsgTrait {
    fn encode(&self) -> Vec<u8>;
    fn decode(buffer: &[u8]) -> Self;
}

trait ByteArrSer {
    fn enc_byte_arr(buffer: &Vec<u8>) -> Vec<u8> {
        let buffer_size: u16 = buffer.len() as u16;
        let size_arr = buffer_size.to_be_bytes(); // converting to bigendian order arr
        let byte_arr: Vec<u8> = size_arr.iter().chain(buffer.iter()).map(|x| *x).collect();
        byte_arr
    }

    fn dec_byte_arr(buffer: &[u8]) -> Vec<u8> {
        // first two digits store the length of the objects
        let len: u16 = ((buffer[0] as u16) << 8) | (buffer[1] as u16);
        // taking the slice which contains encoded object
        let byte_obj = &buffer[2..(2 + len as usize)];
        byte_obj.to_vec()
    }
}

#[derive(Debug)]
struct MemDB {
    data: HashMap<MemKey, MemVal>,
    _init_time: Instant
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum ResMemMsg { 
    KeyNotFound(String),
    KeySaved(String),
    KeyValue(String),
    FailToDes(String),
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum ReqMemMsg {
    StoreKeyVal(String, String),
    FetchKey(String),
}

impl ByteArrSer for ReqMemMsg {}
impl ByteArrSer for ResMemMsg {}

impl ReqMemMsg {
    // find some way to remove duplication of encode/decode
    fn encode(&self) -> Vec<u8> {
        let send_data = bincode::serialize(&self).unwrap();
        Self::enc_byte_arr(&send_data)
    }

    fn decode(buffer: &[u8]) -> Option<Self> {
        // convert this to result type
        // the client should still get an option
        // actually i have changed my mind i think
        // it should be a result wrapping an option
        // key not found is not error which decode or network 
        // failure is
        let byte_obj = Self::dec_byte_arr(&buffer);
        let des: ReqMemMsg = bincode::deserialize(&byte_obj).unwrap();
        Some(des)
    }
}

impl ResMemMsg {
    fn encode(&self) -> Vec<u8> {
        let send_data = bincode::serialize(&self).unwrap();
        Self::enc_byte_arr(&send_data)
    }
    fn decode(buffer: &[u8]) -> Option<Self> {
        let byte_obj = Self::dec_byte_arr(&buffer);
        let des: ResMemMsg = bincode::deserialize(&byte_obj).unwrap();
        Some(des)
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
        // update _last_modified for MemKey when key already exists
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
}

impl MemVal {
    pub fn new (value: String) -> MemVal {
        MemVal { value }
    }
}

#[derive(Debug)]
struct MemKey {
    value: String,
    _created: Instant,
    _last_modified: Instant
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
            _created: Instant::now(),
            _last_modified: Instant::now()
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
            let dec = ReqMemMsg::decode(&buffer[..]);
            let resp: ResMemMsg = match dec {
                None => {
                    ResMemMsg::FailToDes("not of acceptable request message format".to_string())
                }
                Some(decoded) => {
                    match decoded {
                        ReqMemMsg::StoreKeyVal(key, val) => {
                            self.mem.store(key.to_string(), val.to_string());
                            ResMemMsg::KeySaved("Key:Val saved in mem".to_string())
                        },
                        ReqMemMsg::FetchKey(key) => {
                            let val = self.mem.fetch(key.to_string());
                            match val {
                                Some(key_value) => ResMemMsg::KeyValue(key_value.to_string()),
                                None => ResMemMsg::KeyNotFound("key not found".to_string())
                            }
                        }
                    }
                    
                }
            };
            stream.write(&resp.encode()).unwrap();
            println!("Current store {:?}", self.mem);
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

    fn send(&mut self, msg: &[u8]) {
        self.stream.write(msg).unwrap();
    }

    fn recv(&mut self) -> Option<ResMemMsg> {
        let mut buffer = [0; 512];
        self.stream.read(&mut buffer).unwrap();
        let buffer = buffer.to_vec();
        let decoded = ResMemMsg::decode(&buffer);
        decoded
    }

    pub fn fetch (&mut self, key: &str) -> String {
        let msg = ReqMemMsg::FetchKey(key.to_string());
        let msg_byte_arr = msg.encode();
        self.send(&msg_byte_arr[..]);
        if let Some(resp) = self.recv() {
            return match resp {
                ResMemMsg::KeyNotFound(msg) => msg,
                ResMemMsg::KeyValue(val) => val,
                _ => "some problem".to_string()
            };
        };
        "no idea; but something bad happened".to_string()
    }

    pub fn store (&mut self, key: &str, val: &str) -> String {
        let msg = ReqMemMsg::StoreKeyVal(key.to_string(),  val.to_string());
        let msg_byte_arr = msg.encode();
        self.send(&msg_byte_arr[..]);
        if let Some(resp) = self.recv() {
            println!("resp of store is {:?}", resp);
            return match resp {
                ResMemMsg::KeySaved(msg) => msg,
                _ => "some problem".to_string()
            };
        };
        "no idea; but something bad happened".to_string()
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
        let resp = mem_client.fetch(&key);
        println!("val is : {:?}", resp);
    }

    if let Some(matches) = matches.subcommand_matches("store") {
        let key = matches.value_of("key").unwrap();
        let val = matches.value_of("val").unwrap();
        let resp = mem_client.store(&key, &val);
        println!("val is : {:?}", resp);
    }
}
