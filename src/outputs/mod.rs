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

use outputs::syslog::{SyslogConfig, Syslog};
use outputs::elasticsearch::{ Elasticsearch };
use outputs::server::{ Server };
use enums::Config;

mod syslog;
mod elasticsearch;
mod server;


#[derive(Debug, Serialize, Deserialize)]
pub struct OutputsConfig {
    pub syslog : Option<Vec<SyslogConfig>>,
    pub elasticsearch : Option<String>,
    pub zerotrust_endpoint : Option<String>,
}

pub trait Output {
    fn process_open_connection(&mut self, &str);
    fn process_close_connection(&mut self, &str);
}


pub fn create(config : &Config) -> Result<Vec<Box<Output>>, String> {
        let mut outputs : Vec<Box<Output>> = Vec::new();
        if let Some(ref config) = config.outputs.syslog {
            for output in config.iter() {
            match output {
                    SyslogConfig::Localhost => {
                        info!("adding localhost syslog output");
                        let syslog = Syslog::local()?;
                        outputs.push(Box::new(syslog));
                    },
                    SyslogConfig::TCP{address, port} => {
                        info!("adding tcp syslog output");
                        let syslog = Syslog::tcp(address, *port)?;
                        outputs.push(Box::new(syslog));
                    },
                    SyslogConfig::UDP{address, port} => {
                        info!("adding udp syslog output");
                        let syslog = Syslog::udp(address, *port)?;
                        outputs.push(Box::new(syslog));
                    },
                };
            }
        }

        if let Some(ref config) = config.outputs.elasticsearch {
            info!("adding elasticsearch output: {}", config);
            let elasticsearch = Elasticsearch::new(config)?;
            outputs.push(Box::new(elasticsearch));
        }

        if let Some(ref endpoint_config) = config.outputs.zerotrust_endpoint {
            info!("adding server output: {} / {:?} / {:?}", endpoint_config, config.name, config.uuid);
            let server = Server::new(&config.name, &config.uuid, endpoint_config)?;
            outputs.push(Box::new(server));
        }

        Ok(outputs)
}

#[cfg(test)]
mod tests {
    use std::net::{ Ipv4Addr, TcpListener, UdpSocket };
    use enums;
    use filters;

    #[test]
    fn test_create_failed() {
        let mut vec = Vec::new();
        vec.push( super::SyslogConfig::Localhost );
        vec.push( super::SyslogConfig::TCP {
            address : Ipv4Addr::new(127, 0, 0, 1),
            port: 7233
        });

        vec.push( super::SyslogConfig::UDP {
            address : Ipv4Addr::new(127, 0, 0, 1),
            port: 7233
        });

        let config = enums::Config {
            directory: None,
            name: None,
            uuid: None,
            outputs: super::OutputsConfig {
                syslog: Some(vec),
                elasticsearch: None,
                zerotrust_endpoint: None,
            },
            filters: filters::FiltersConfig {
                non_process_connections : false,
                dns_requests: false,
                zerotrust_track_connections : false
            }
        };

        let config = super::create(&config);
        assert!(config.is_err());
    }

    #[test]
    fn test_create_success() {
        let _tcp = TcpListener::bind("127.0.0.1:7232").unwrap();
        let _udp = UdpSocket::bind("127.0.0.1:7232").unwrap();

        let mut vec = Vec::new();
        vec.push( super::SyslogConfig::Localhost );
        vec.push( super::SyslogConfig::TCP {
            address : Ipv4Addr::new(127, 0, 0, 1),
            port: 7232
        });
        vec.push( super::SyslogConfig::UDP {
            address : Ipv4Addr::new(127, 0, 0, 1),
            port: 7232
        });

        let config = enums::Config {
            directory: None,
            name: None,
            uuid: None,
            outputs: super::OutputsConfig {
                syslog: Some(vec),
                elasticsearch: None,
                zerotrust_endpoint: None,
            },
            filters: filters::FiltersConfig {
                non_process_connections : false,
                dns_requests: false,
                zerotrust_track_connections : false
            }
        };

        let config = super::create(&config);
        assert!(!config.is_err());
    }


}
