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

use std::net::Ipv4Addr;
use std::io;
use std::thread;
use std::time;
use std::u32;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use users::{Users, UsersCache};
use proc_chomper::{ProcChomper};
use enums::{ Protocol, State };
use proc::{Proc};
use conn_track;
use chrono::prelude::*;
use uuid::Uuid;

pub fn generate_hash(
    protocol : &str,
    source: &Ipv4Addr,
    source_port: &u16,
    destination: &Ipv4Addr,
    destination_port: &u16
) -> u64 {
    let mut s = DefaultHasher::new();
    protocol.hash(&mut s);
    source.hash(&mut s);
    source_port.hash(&mut s);
    destination.hash(&mut s);
    destination_port.hash(&mut s);
    s.finish()
}


#[derive(Debug, Serialize)]
pub enum Payload {
    Open(OpenConnection),
    Close(CloseConnection),
}


#[derive(Debug, Serialize)]
pub struct OpenConnection {
    pub hash: i64,
    pub uuid : Uuid,
    pub agent: Uuid,
    pub timestamp : String,
    pub protocol : Protocol,
    pub source: Ipv4Addr,
    pub destination : Ipv4Addr,
    pub source_port : u16,
    pub destination_port : u16,
    pub username : String,
    pub uid : u16,
    pub program_details : Option<Program>,
}

#[derive(Debug, Serialize)]
pub struct CloseConnection {
    pub hash: i64,
    pub agent: Uuid,
    pub uuid: Option<Uuid>,
    pub timestamp : String,
    pub protocol : Protocol,
    pub source: Ipv4Addr,
    pub destination : Ipv4Addr,
    pub source_port : u16,
    pub destination_port : u16,
}

#[derive(Debug, Serialize)]
pub struct Program {
    pub inode: u32,
    pub pid: u32,
    pub process_name : String,
    pub command_line : Vec<String>,
}

pub struct Parser {
    user_cache: UsersCache,
    tcp_chomper : ProcChomper,
    udp_chomper : ProcChomper,
    proc: Proc,
    agent : Uuid,
}

impl Parser {
    pub fn new(agent : Uuid) -> Result<Parser, io::Error> {
        let tcp_chomper = ProcChomper::new(Protocol::TCP)?;
        let udp_chomper = ProcChomper::new(Protocol::UDP)?;
        let user_cache = UsersCache::new();
        let proc = Proc::new()?;

        Ok(Parser {
            user_cache,
            tcp_chomper,
            udp_chomper,
            proc,
            agent,
        })
    }

    pub fn parse(&mut self, con : conn_track::Connection) -> Option<Payload> {
        let state = con.state;

        match con.details.protocol {
            conn_track::ProtoDetails::IP{ protocol, source_port, destination_port } => self.parse_ip_connection(state, protocol, con.details.source, con.details.destination, source_port, destination_port),
            _ => {
                trace!("protocol isn't IP, dropping it");
                None
            },
        }
    }

    fn parse_ip_connection(&mut self, state: State, protocol: Protocol, source : Ipv4Addr, destination : Ipv4Addr, source_port : u16, destination_port : u16) -> Option<Payload> {
        let chomper =  match protocol {
            Protocol::UDP => &self.udp_chomper,
            Protocol::TCP => &self.tcp_chomper,
        };

        let mut inode = 0;
        let mut uid = 0;
        let mut username = String::new();

        while inode == 0 {
            let _ = chomper.update();
            if let Some(connection) = chomper.find(&source, source_port) {
                inode = connection.inode;
                uid = connection.uid;
                if let Some(user) = self.user_cache.get_user_by_uid(uid as u32) {
                    username = user.name().to_string();
                }

                if inode == 0 {
                    // We're too quick the socket table hasn't been updated yet.
                    thread::sleep(time::Duration::from_millis(2));
                }
            } else {
                inode = <u32>::max_value();
            }
        }

        let program_details = match inode == <u32>::max_value() {
            true => None,
            false => {
                match self.proc.get(inode) {
                    Some(process) => {
                        let pid : u32 = process.stat.pid as u32;
                        let process_name = process.stat.comm.clone();
                        let command_line = process.cmdline().unwrap();

                        Some(Program {
                            inode,
                            pid,
                            process_name,
                            command_line
                        })
                    },
                    None => {
                        None
                    }
                }
            }
        };

        let timestamp = Utc::now().to_rfc3339();

        // This is used to tie connections together.
        let hash =  generate_hash(
            &protocol.to_string(),
            &source,
            &source_port,
            &destination,
            &destination_port) as i64;

        let uuid = Uuid::new_v4();
        let agent = self.agent.clone();
        let payload = match state {
            State::New => Some(
                Payload::Open(OpenConnection {
                    hash,
                    uuid,
                    agent,
                    timestamp,
                    protocol,
                    source,
                    destination,
                    source_port,
                    destination_port,
                    username,
                    uid,
                    program_details,
                })),
            State::Destroy => Some(
                Payload::Close(CloseConnection {
                    hash,
                    uuid: None,
                    agent,
                    timestamp,
                    protocol,
                    source,
                    destination,
                    source_port,
                    destination_port,
                })),
            _ => None,
        };

        payload

    }

}
