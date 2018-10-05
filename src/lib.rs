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

extern crate libc;
extern crate crslmnl as mnl;

#[macro_use]
extern crate log;
extern crate core;
extern crate users;
extern crate procfs;
extern crate syslog;
extern crate sys_info;
extern crate reqwest;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate serde_yaml;

extern crate tempfile;

use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use std::thread;
use std::fs::File;
use std::io::prelude::*;


use parser::Parser;
use conn_track::Conntrack;
use outputs::OutputsConfig;

mod conn_track;
mod proc_chomper;
mod parser;
mod proc;
pub mod outputs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub outputs : OutputsConfig,
    pub include_non_process_connections : bool,
    pub include_dns_request : bool,
}

#[derive(Debug, Serialize)]
pub enum Protocol {
    UDP,
    TCP,
}

#[derive(Debug, Serialize, PartialEq)]
pub enum State {
    New,
    Destroy,
    Unknown,
}

pub struct NoTrack {
    config : Config,
    outputs : Vec<Box<outputs::Output>>,
}

impl NoTrack {
    pub fn from_str(config: &str) -> Result<NoTrack, String> {
        let config : Config = match serde_yaml::from_str(config) {
            Ok(x) => x,
            Err(err) => {
                println!("Unable to parse config: {}", err);
                return Err(String::from("unable to parse config"));
            }
        };

        NoTrack::new(config)
    }

    pub fn from_file(name: &str) -> Result<NoTrack, String> {
        let mut file = match File::open(name) {
            Ok(x) => x,
            Err(_err) => return Err(String::from("unable to open config file")),
        };

        let mut contents = String::new();
        if let Err(_) = file.read_to_string(&mut contents) {
            return Err(String::from("unable to read config file"));
        }

        NoTrack::from_str(&contents)
    }

    pub fn new(config: Config) -> Result<NoTrack, String> {
        let outputs = outputs::create(&config.outputs)?;
        Ok(NoTrack {
            config : config,
            outputs :  outputs
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut tracker=  match Conntrack::new() {
            Ok(x) => x,
            Err(_err) => return Err(String::from("unable to bind to conntrack, please check permissions")),
        };

        let mut parser = match Parser::new() {
            Ok(x) => x,
            Err(_err) => return Err(String::from("unable to parse process descriptors, please check permissions")),
        };

        let (mut tx, rx) : (Sender<conn_track::Connection>, Receiver<conn_track::Connection>) = channel();

        thread::spawn(move || {
            info!("starting conntrack");
            tracker.start(&mut tx);
        });

        info!("starting main loop");
        loop {
            if let Ok(con) = rx.recv() {
                trace!("recieved {:?} from channel, parsing", con);
                if let Some(payload) = parser.parse(con) {
                    if  ! self.config.include_non_process_connections &&
                        payload.program_details.is_none() &&
                        payload.state == State::New
                    {
                        debug!("dropping payload as it doesn't include process information");
                    } else {
                        if ! self.config.include_dns_request &&
                            ( payload.destination_port == 53 || payload.destination_port == 5353)
                        {
                            debug!("dropping payload as it's a DNS request");
                        } else {
                            let json = serde_json::to_string(&payload).unwrap();
                            trace!("created json payload: {}", json);
                            for output in &mut self.outputs {
                                output.process(&json);
                            }
                        }
                    }
                } else {
                    debug!("recieved none, dropping packet");
                }
            } else {
                warn!("closing application");
                break;
            }
        }

        Ok(())
    }


    pub fn dump_config(&self) -> Result<(), String> {
        dump_config(&self.config)
    }

}

pub fn dump_config(config: &Config) -> Result<(), String> {
    let config = match serde_yaml::to_string(config) {
        Ok(x) => x,
        Err(_err) => return Err(String::from("Unable to dump config!")),
    };

    println!("{}", config);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_string() -> String {
        let string = String::from("---\noutputs:\n  syslog: []\ninclude_non_process_connections: false\ninclude_dns_request: false");
        return string;
    }

    #[test]
    fn test_dump_config_success() {
        let config = Config {
            outputs : OutputsConfig {
                syslog : Vec::new(),
                elasticsearch : None,
            },
            include_non_process_connections: false,
            include_dns_request: false,
        };


        assert!(!dump_config(&config).is_err());
    }

    #[test]
    fn test_from_str_fail() {
        assert!(NoTrack::from_str("").is_err());
    }

    #[test]
    fn test_from_str_success() {
        let string = config_string();
        assert!(!NoTrack::from_str(&string).is_err());
    }

    #[test]
    fn test_from_file_fail() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let file = temp_file.path().to_str().unwrap();
        assert!(NoTrack::from_file(file).is_err());
    }

    #[test]
    fn test_from_file_success() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        write!(&temp_file, "{}", config_string()).unwrap();
        assert!(!NoTrack::from_file(path).is_err());
    }

}
