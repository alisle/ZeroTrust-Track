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
extern crate simple_logger;
extern crate chrono;
extern crate uuid;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate serde_yaml;
extern crate rand;
extern crate tempfile;

use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use std::thread;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use parser::{ Parser, Payload };
use conn_track::Conntrack;
use rand::Rng;
use enums::{ Config };
use filters::{ Filter };
use state::{ State };
mod conn_track;
mod proc_chomper;
mod parser;
mod proc;
mod state;

pub mod outputs;
pub mod enums;
pub mod filters;

use uuid::Uuid;
use std::fs;
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize)]
struct NameTuple {
    name: Option<String>,
    uuid: Option<Uuid>
}

pub struct NoTrack {
    pub config : Config,
    filter: Filter,
    outputs : Vec<Box<outputs::Output>>,
}

impl NoTrack {
    pub fn from_str(config: &str, data_directory: Option<&str>) -> Result<NoTrack, String> {
        let mut config : Config = match serde_yaml::from_str(config) {
            Ok(x) => x,
            Err(err) => {
                error!("Unable to parse config: {}", err);
                return Err(String::from("unable to parse config"));
            }
        };

        let directory = match  data_directory {
            Some(directory) => String::from(directory),
            None => {
                match config.directory {
                    Some(directory) => directory,
                    None => return Err(String::from("no data directory defined")),
                }
            }
        };

        if check_directory(&directory) == false {
            return Err(String::from("data directory defined, does not exist"));
        }


        config = Config {
            directory: Some(String::from(directory)),
            .. config
        };

        NoTrack::new(config)
    }

    pub fn from_file(name: &str, data_directory : Option<&str>) -> Result<NoTrack, String> {
        let mut file = match File::open(name) {
            Ok(x) => x,
            Err(_err) => return Err(String::from("unable to open config file")),
        };

        let mut contents = String::new();
        if let Err(_) = file.read_to_string(&mut contents) {
            return Err(String::from("unable to read config file"));
        }

        NoTrack::from_str(&contents, data_directory)
    }

    pub fn new(config: Config) -> Result<NoTrack, String> {
        let outputs = outputs::create(&config.outputs)?;
        let filter = Filter::new(config.filters)?;
        let config = populate_config(config);

        Ok(NoTrack {
            config : config,
            outputs :  outputs,
            filter: filter,
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

        let mut state = match State::new() {
            Ok(x) => x,
            Err(_err) => return Err(String::from("unable to start the state module")),
        };


        thread::spawn(move || {
            info!("starting conntrack");
            tracker.start(&mut tx);
        });


        info!("starting main loop");
        loop {
            if let Ok(con) = rx.recv() {
                trace!("recieved {:?} from channel, parsing", con);
                if let Some(payload) = parser.parse(con) {
                    if ! self.filter.apply(&payload) {
                        let payload = state.transform(payload);
                        let json = match payload {
                            Payload::Open(ref connection)  => serde_json::to_string(connection).unwrap(),
                            Payload::Close(ref connection) => serde_json::to_string(connection).unwrap(),
                        };

                        trace!("created json payload: {}", json);
                        for output in &mut self.outputs {
                            match payload {
                                Payload::Open(_) => output.process_open_connection(&json),
                                Payload::Close(_) => output.process_close_connection(&json),
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

fn check_directory(directory : &str) -> bool {
    Path::new(directory).exists()
}

fn load_names(file : &str) -> Vec<String> {
    let mut vec = Vec::new();
    if ! Path::new(file).exists() {
        warn!("name file doesn't exist");
    } else {
        if let Ok(file) = File::open(file) {
            let buffer  = BufReader::new(file);
            for line in buffer.lines() {
                match line {
                    Ok(x) => vec.push(x),
                    Err(err) => warn!("skipping loading line {}", err)
                }
             }
        };
    }

     vec
}

fn load_uuid_name_tuple(file: &str) -> NameTuple {
    if ! Path::new(file).exists() {
        warn!("name type file doesn't exist");
    } else {
        if let Ok(mut file) = File::open(file) {
            let mut contents = String::new();
            match file.read_to_string(&mut contents) {
                Ok(_) => {
                    if let Ok(tuple) = serde_yaml::from_str(&contents) {
                        return tuple;
                    }
                },
                _ => ()
            }
        }
    }

    return NameTuple { name: None, uuid: None };
}

fn save_uuid_name_tuple(file: &str, tuple : &NameTuple) -> Result<(), String>{
    let tuple_string = match serde_yaml::to_string(tuple) {
        Ok(x) => x,
        Err(_err) => return Err(String::from("Unable to name tuple!")),
    };

    if let Err(_) = fs::write(file, tuple_string) {
        return Err(String::from("unable to save name tuple"));
    }

    Ok(())
}

fn populate_config(config: Config) -> Config {

    let directory = match config.directory {
        Some(directory) => directory,
        None => {
            warn!("No working directory set, using /tmp");
            String::from("/tmp")
        }
    };

    let tuple_file_name = format!("{}/{}", directory, "/name_tuple.yaml");
    let names_file_name = format!("{}/{}", directory, "/names.txt");
    let tuple = load_uuid_name_tuple(&tuple_file_name);
    let names = load_names(&names_file_name);

    let uuid = match config.uuid {
        Some(uuid) => uuid,
        None => {
            match tuple.uuid {
                Some(uuid) => uuid,
                None => Uuid::new_v4()
            }
        }
    };

    let name = match config.name {
        Some(name) => name,
        None => {
            let name = match tuple.name {
                Some(name) => name,
                None => {
                    let name = match rand::thread_rng().choose(&names) {
                        Some(name) => name.clone(),
                        None => String::from("unknown"),
                    };

                    name
                },
            };

            name
        },
    };

    if let Err(err) = save_uuid_name_tuple(&names_file_name, &NameTuple { name: Some(name.clone()), uuid: Some(uuid.clone())}) {
        warn!("unable to save file tuple {}", err);
    }

    Config {
        name: Some(name),
        uuid: Some(uuid),
        directory: Some(directory),
        .. config
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
    use filters::FiltersConfig;
    use outputs::OutputsConfig;
    use tempfile::{tempdir};
    use uuid::Uuid;

    fn config_string() -> String {
        let string = String::from("---\ndirectory: /tmp\noutputs:\n  syslog: []\nfilters:\n  non_process_connections: true\n  dns_requests : true\n  notrust_track_connections: true");
        return string;
    }

    fn default_filters() -> FiltersConfig {
        FiltersConfig {
            non_process_connections: true,
            dns_requests : true,
            notrust_track_connections: true,
        }
    }

    fn default_config() -> Config {
        Config {
            directory: None,
            name: None,
            uuid: None,
            outputs : OutputsConfig {
                notrust_endpoint: None,
                syslog : Some(Vec::new()),
                elasticsearch : None,
            },
            filters: default_filters(),
        }
    }


    #[test]
    fn test_dump_config_success() {
        let config = default_config();
        assert!(!dump_config(&config).is_err());
    }

    #[test]
    fn test_from_str_fail() {
        assert!(NoTrack::from_str("", None).is_err());
    }

    #[test]
    fn test_from_str_success() {
        let string = config_string();
        assert!(!NoTrack::from_str(&string, None).is_err());
    }

    #[test]
    fn test_from_file_fail() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let file = temp_file.path().to_str().unwrap();
        assert!(NoTrack::from_file(file, None).is_err());
    }

    #[test]
    fn test_from_file_success() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        write!(&temp_file, "{}", config_string()).unwrap();
        assert!(!NoTrack::from_file(path, None).is_err());
    }

    #[test]
    fn test_override_data_directory_success() {
        let config = config_string();
        let tempdir = tempdir().unwrap();
        let data_directory = tempdir.path();

        let notrack = NoTrack::from_str(&config, data_directory.to_str()).unwrap();
        let directory = notrack.config.directory.unwrap();

        assert_eq!(&directory, data_directory.to_str().unwrap());
    }


    #[test]
    fn test_check_directory_success() {
        let tempdir = tempdir().unwrap();
        let data_directory = tempdir.path();

        assert!(check_directory(data_directory.to_str().unwrap()));
    }

    #[test]
    fn test_check_directory_fail() {
        assert!(!check_directory("/I_like_strange_things"));
    }

    #[test]
    fn test_load_names_success() {
        let names = load_names("resources/names.txt");
        assert!(names.len() > 0);
    }

    #[test]
    fn test_load_names_fail() {
        let names = load_names("resources/Things look strange from down here.txt");
        assert!(names.len() == 0);

    }

    #[test]
    fn test_save_load_uuid_name_tuple_success() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let file = temp_file.path().to_str().unwrap();
        let tuple = NameTuple { name: Some(String::from("I am a name")), uuid: Some(Uuid::new_v4()) };

        save_uuid_name_tuple(file, &tuple).unwrap();

        let new_tuple =  load_uuid_name_tuple(file);
        assert_eq!(tuple.name, new_tuple.name);
        assert_eq!(tuple.uuid, new_tuple.uuid);
    }

    #[test]
    fn test_populate_config_defined_name_uuid()  {
        let uuid = Uuid::new_v4();
        let config = Config {
            name : Some(String::from("I am a name")),
            uuid : Some(uuid.clone()),
            .. default_config()
        };

        let updated_config = populate_config(config);

        assert_eq!(Some(String::from("I am a name")), updated_config.name);
        assert_eq!(Some(uuid), updated_config.uuid);
    }

    #[test]
    fn test_populate_config_undefined_uuid() {
        let config = Config {
            name : Some(String::from("I am a name")),
            uuid : None,
            .. default_config()
        };
        let updated_config = populate_config(config);
        assert_eq!(Some(String::from("I am a name")), updated_config.name, "names are not the same");

        match updated_config.uuid {
            None =>  assert!(false, "uuid is not defined"),
            _ => assert!(true),
        };
    }

    #[test]
    fn test_populate_config_undefined_name() {
        let uuid = Uuid::new_v4();

        let config = Config {
            name : None,
            uuid : Some(uuid.clone()),
            .. default_config()
        };

        let updated_config = populate_config(config);
        assert_eq!(Some(uuid), updated_config.uuid, "uuid's are not the same");
        match updated_config.name {
            None =>  assert!(false, "name is not defined"),
            _ => assert!(true),
        };

    }

}
