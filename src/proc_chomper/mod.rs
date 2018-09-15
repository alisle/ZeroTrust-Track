/*
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *  http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *
 *
 */

use std::io;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;
use std::net::Ipv4Addr;
use std::collections::HashMap;
use std::cell::RefCell;

use super::Protocol;

pub static TCP_LIST: &'static  str = "/proc/net/tcp";
pub static UDP_LIST: &'static str = "/proc/net/udp";

#[derive(Debug, Clone)]
pub struct SocketConnection {
    local_address : Ipv4Addr,
    local_port : u16,
    remote_address : Ipv4Addr,
    remote_port : u16,
    pub uid : u16,
    pub inode : u32
}

#[derive(PartialEq, Eq, Hash)]
struct Key{
    address :Ipv4Addr,
    port: u16
}

pub struct ProcChomper{
    protocol : Protocol,
    map : RefCell<HashMap<Key, SocketConnection>>,
}

impl ProcChomper {
    pub fn new(protocol : Protocol) -> Result<ProcChomper, io::Error> {
        let chomper = ProcChomper {
            protocol,
            map: RefCell::new(HashMap::new()),
        };

        chomper.update()?;
        Ok(chomper)
    }

    pub fn update(&self) -> Result<(), io::Error>{
        let file = match self.protocol {
            Protocol::UDP => File::open(UDP_LIST)?,
            Protocol::TCP => File::open(TCP_LIST)?,
        };

        let reader = BufReader::new(file);
        let mut map : HashMap<Key, SocketConnection> = HashMap::new();

        for (num, line) in reader.lines().enumerate() {
            let line = line.unwrap();

            if num == 0 {
                continue;
            }

            if let Some(connection) = parse_connection(&line) {
                map.insert(Key {
                    address: connection.local_address.clone(),
                    port: connection.local_port
                }, connection.clone());

                map.insert(Key {
                    address: connection.remote_address.clone(),
                    port: connection.remote_port
                }, connection.clone());
            }
        }

        self.map.replace(map);
        Ok(())
    }

    pub fn find(&self, address : &Ipv4Addr, port : u16) -> Option<SocketConnection> {
        let map = self.map.borrow();

        match map.get(&Key {
            address: address.clone(),
            port
        }) {
            Some(connection) => Some(connection.clone()),
            None => None,
        }
    }
}

fn parse_connection(line: &str) -> Option<SocketConnection> {
    let split = line.split(" ");
    let mut split = split.collect::<Vec<&str>>();
    split.retain(|&x| x.len() != 0);

    let mut local_address : Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
    let mut local_port : u16 = 0;
    let mut remote_address : Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
    let mut remote_port : u16 = 0;
    let mut uid : u16 = 0;
    let mut inode : u32 = 0;


    for(count, item) in split.iter().enumerate() {
        match count {
            1 => {
                if let Some(tuple) = split_address(item) {
                    let address = u32::from_be(u32::from_str_radix(&tuple.0, 16).unwrap());
                    local_address = Ipv4Addr::from(address);
                    local_port = u16::from_str_radix(&tuple.1, 16).unwrap();
                }
            },
            2 => {
                if let Some(tuple) = split_address(item) {
                    let address = u32::from_be(u32::from_str_radix(&tuple.0, 16).unwrap());
                    remote_address = Ipv4Addr::from(address);
                    remote_port = u16::from_str_radix(&tuple.1, 16).unwrap();
                }
            },
            7 => { uid = item.parse().unwrap(); },
            9 => { inode = item.parse().unwrap(); },
            _ => ()
        }
    }

    Some(SocketConnection {
        local_address,
        local_port,
        remote_address,
        remote_port,
        uid,
        inode
    })
}

fn split_address(pair : &str) -> Option<(String, String)> {
    let tuple = pair.split(":");
    let tuple = tuple.collect::<Vec<&str>>();

    if tuple.len() < 2 {
        return None;
    }

    Some((String::from(tuple[0]), String::from(tuple[1])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_address_failure() {
        let tuple = split_address("I have no breaks");
        assert!(!tuple.is_some())
    }

    #[test]
    fn test_split_address_success() {
        let tuple = split_address("I have:breaks");
        match tuple {
            Some(tuple) => {
                assert_eq!("I have", tuple.0);
                assert_eq!("breaks", tuple.1);
            },
            None => {
                assert!(tuple.is_some());
            }
        }


    }

    #[test]
    fn test_parse_connection_success() {
        let string = "   3: 669010AC:0016 019010AC:D575 01 00000000:00000000 02:000577BD 00000000     0        0 1227937 2 0000000000000000 20 4 25 2 2                    ";
        let payload = parse_connection(string);
        match payload {
            Some(payload) => {
                assert_eq!(payload.local_address, Ipv4Addr::new(172,16,144,102));
                assert_eq!(payload.local_port, 22);
                assert_eq!(payload.remote_address, Ipv4Addr::new(172,16,144,1));
                assert_eq!(payload.remote_port, 54645);
                assert_eq!(payload.uid, 0);
                assert_eq!(payload.inode, 1227937);
            },
            None => {
                assert!(payload.is_some());
            }
        }

    }
}
