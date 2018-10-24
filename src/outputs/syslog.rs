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


use syslog;
use syslog::{Formatter3164, Facility};
use std::net::Ipv4Addr;
use libc::{getpid};
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use std::thread;
use sys_info;

use outputs::{ Output };


#[derive(Debug, Serialize, Deserialize)]
pub enum SyslogConfig {
    Localhost,
    TCP{ address: Ipv4Addr, port : u16 },
    UDP{ address: Ipv4Addr, port: u16 },
}
pub struct Syslog {
    tx : Sender<String>,
}

impl Syslog {
    pub fn local() -> Result<Syslog, String> {
        let formatter = create_formatter();
        let (tx, rx) : (Sender<String>, Receiver<String>) = channel();
        let mut writer = match syslog::unix(formatter) {
            Ok(writer) => writer,
            Err(_) => return Err(String::from("unable to start localhost syslog"))
        };

        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(message) => {
                        if let Err(_) = writer.err(message) {
                            error!("unable to write to syslog");
                        }
                    },
                    Err(err) => {
                        error!("closing thread: {}", err);
                        break;
                    }
                };
            }
        });

        Ok(Syslog {
            tx,
        })
    }

    pub fn udp(address : &Ipv4Addr, port: u16) -> Result<Syslog, String> {
        let formatter = create_formatter();
        let (tx, rx) : (Sender<String>, Receiver<String>) = channel();
        let connect_string = address.to_string() + ":" + &port.to_string();

        let mut writer = match syslog::udp(formatter,  "127.0.0.1:3514", &connect_string) {
            Ok(writer) => writer,
            Err(_) => return Err(String::from("unable to start UDP syslog sender"))
        };

        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(message) => {
                        if let Err(_) = writer.err(message) {
                            error!("unable to write to syslog");
                        }
                    },
                    Err(err) => {
                        error!("closing thread: {}", err);
                        break;

                    }
                };
            }
        });

        Ok(Syslog {
            tx,
        })
    }


    pub fn tcp(address : &Ipv4Addr, port : u16 ) -> Result<Syslog, String> {
        let formatter = create_formatter();
        let (tx, rx) : (Sender<String>, Receiver<String>) = channel();
        let connect_string = address.to_string() + ":" + &port.to_string();

        let mut writer = match syslog::tcp(formatter,  connect_string) {
            Ok(writer) => writer,
            Err(_) => return Err(String::from("unable to start TCP syslog sender"))
        };

        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(message) => {
                        if let Err(_) = writer.err(message) {
                            error!("unable to write to syslog");
                        }
                    },
                    Err(err) => {
                        error!("closing thread: {}", err);
                        break;
                    }
                };
            }
        });

        Ok(Syslog {
            tx,
        })
    }
}

impl Output for Syslog {
    fn process_open_connection(&mut self, message: &str) {
        let _ = self.tx.send(message.to_string());
    }

    fn process_close_connection(&mut self, message: &str) {
        let _ = self.tx.send(message.to_string());
    }

}

fn create_formatter() -> Formatter3164 {
    return Formatter3164  {
        facility: Facility::LOG_USER,
        hostname: match sys_info::hostname() {
            Ok(name) => Some(name.to_string()),
            _ => None
        },
        process: "notrust-tracker".into(),
        pid: unsafe { getpid() },
    };
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::net::UdpSocket;

    use super::*;

    #[test]
    fn test_create_syslog_unix() {
        if let Ok(mut writer) = Syslog::local() {
            writer.process("Hello people");
        } else {
            assert!(false, "unable to create syslog client");
        }

    }

    #[test]
    fn test_create_syslog_tcp() {
        let _listener = TcpListener::bind("127.0.0.1:3514").unwrap();
        if let Ok(mut writer) = Syslog::tcp(&Ipv4Addr::new(127, 0, 0, 1), 3514) {
            writer.process("Hello people");
        } else {
            assert!(false, "unable to create the syslog client");
        }

    }

    #[test]
    fn test_create_syslog_udp() {
        let _listener = UdpSocket::bind("127.0.0.1:5514").unwrap();
        if let Ok(mut writer) = Syslog::udp(&Ipv4Addr::new(127, 0, 0, 1), 5514) {
            writer.process("Hello people");
        } else {
            assert!(false, "unable to create the syslog client");
        }
    }

}
