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
use outputs::elasticsearch:: { Elasticsearch };

mod syslog;
mod elasticsearch;


#[derive(Debug, Serialize, Deserialize)]
pub struct OutputsConfig {
    pub syslog : Vec<SyslogConfig>,
    pub elasticsearch : Option<String>,
}

pub trait Output {
    fn process(&mut self, &str);
}


pub fn create(config : &OutputsConfig) -> Result<Vec<Box<Output>>, String> {
        let mut outputs : Vec<Box<Output>> = Vec::new();
        for output in config.syslog.iter() {
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

        if let Some(ref config) = config.elasticsearch {
            info!("adding elasticsearch output: {}", config);
            let elasticsearch = Elasticsearch::new(config)?;
            outputs.push(Box::new(elasticsearch));
        }

        Ok(outputs)
}

#[cfg(test)]
mod tests {
    use std::net::{ Ipv4Addr, TcpListener, UdpSocket };
    
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

        let config = super::OutputsConfig {
            syslog: vec,
            elasticsearch: None,
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
        let config = super::OutputsConfig {
            syslog: vec,
            elasticsearch: None,
        };

        let config = super::create(&config);
        assert!(!config.is_err());
    }


}
