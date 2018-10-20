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

use std::collections::HashSet;
use libc::{ getpid };
use enums::{ State };
use parser::{ Payload };

 #[derive(Debug, Serialize, Deserialize, Copy, Clone)]
 pub struct FiltersConfig {
     pub non_process_connections : bool,
     pub dns_requests : bool,
     pub notrust_track_connections: bool,
 }

#[derive(Clone)]
 pub struct Filter {
     config : FiltersConfig,
     filtered : HashSet<i64>,
     pid: u32,
 }


impl Filter {
    pub fn new(config: FiltersConfig) -> Result<Filter, String> {
        Ok(Filter {
            config: config,
            pid : unsafe { getpid() } as u32,
            filtered: HashSet::new(),
        })
    }

    pub fn apply(&mut self, payload: &Payload) -> bool {
        if payload.state == State::Destroy &&
            self.filtered.contains(&payload.hash)
        {
            trace!("removing payload from filter hash set");
            self.filtered.remove(&payload.hash);

            return true;
        }

        if  self.config.non_process_connections &&
            payload.program_details.is_none() &&
            payload.state == State::New
        {
            trace!("dropping payload as it doesn't include process information");
            self.filtered.insert(payload.hash);
            return true;
        }

        if self.config.dns_requests &&
            ( payload.destination_port == 53 || payload.destination_port == 5353)
        {
            trace!("dropping payload as it's a DNS request");
            self.filtered.insert(payload.hash);
            return true;
        }

        if self.config.notrust_track_connections {
            if let Some(ref details) = payload.program_details {
                if details.pid == self.pid {
                    trace!("dropping payload is the pid is the same as ours");
                    self.filtered.insert(payload.hash);
                    return true;
                }
            }
        }


        trace!("allowing payload");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use enums::{ Protocol, State };
    use std::net::Ipv4Addr;
    use parser::{ Program, generate_hash };
    use chrono::prelude::*;



    fn default_payload() -> Payload {
        Payload {
            state: State::New,
            hash: generate_hash(
                &Protocol::TCP.to_string(),
                &Ipv4Addr::new(127, 0, 0, 1),
                &22,
                &Ipv4Addr::new(127, 0, 0, 1),
                &22
            ) as i64,
            timestamp: Utc::now().to_rfc3339(),
            protocol: Protocol::TCP,
            source_port : 22,
            source: Ipv4Addr::new(127, 0, 0, 1),
            destination_port : 22,
            destination : Ipv4Addr::new(127, 0, 0, 1),
            username : String::from("hello"),
            uid: 10,
            program_details : None,
        }
    }

    fn default_filters() -> FiltersConfig {
        FiltersConfig {
            non_process_connections: true,
            dns_requests : true,
            notrust_track_connections: true,
        }
    }


    #[test]
    fn test_filter_set_true() {
        let  mut filter = Filter::new(FiltersConfig {
            non_process_connections: true,
           .. default_filters()
        }).unwrap();

        let payload = default_payload();
        assert_eq!(true, filter.apply(&payload));

        let payload = Payload {
            state : State::Destroy,
            .. default_payload()
        };
        assert_eq!(true, filter.apply(&payload));

        let payload = Payload {
            state : State::Destroy,
            .. default_payload()
        };
        assert_eq!(false, filter.apply(&payload));
    }




    #[test]
    fn test_filter_include_non_process_connections_false() {
        let mut filter = Filter::new(FiltersConfig {
            non_process_connections: false,
           .. default_filters()
        }).unwrap();

        let payload = default_payload();
        assert_eq!(false, filter.apply(&payload));
    }

    #[test]
    fn test_filter_include_non_process_connections_true() {
        let  mut filter = Filter::new(FiltersConfig {
            non_process_connections: true,
           .. default_filters()
        }).unwrap();

        let payload = default_payload();
        assert_eq!(true, filter.apply(&payload));
    }

    #[test]
    fn test_filter_include_dns_request_false() {
        let  mut filter = Filter::new(FiltersConfig {
            non_process_connections: false,
            dns_requests : false,
           .. default_filters()
        }).unwrap();

        let payload = Payload {
            source_port: 53,
            destination_port: 53,
            .. default_payload()
        };

        assert_eq!(false, filter.apply(&payload));
    }

    #[test]
    fn test_filter_include_dns_request_true() {
        let mut filter = Filter::new(FiltersConfig {
            non_process_connections: false,
            dns_requests : true,
           .. default_filters()
        }).unwrap();

        let payload = Payload {
            source_port: 53,
            destination_port: 53,
            .. default_payload()
        };

        assert_eq!(true, filter.apply(&payload));
    }


    #[test]
    fn test_filter_notrust_track_connections_true() {
        let mut filter = Filter::new(FiltersConfig {
            notrust_track_connections : true,
           .. default_filters()
        }).unwrap();

        let payload = Payload {
            program_details : Some(Program {
                    inode: 0,
                    pid: unsafe { getpid() } as u32,
                    process_name : String::from("I am a program"),
                    command_line : Vec::new()
            }),
            .. default_payload()
        };

        assert_eq!(true, filter.apply(&payload));
    }

    #[test]
    fn test_filter_notrust_track_connections_false() {
        let mut filter = Filter::new(FiltersConfig {
            notrust_track_connections : false,
           .. default_filters()
        }).unwrap();

        let payload = Payload {
            program_details : Some(Program {
                    inode: 0,
                    pid: 100,
                    process_name : String::from("I am a program"),
                    command_line : Vec::new()
            }),
            .. default_payload()
        };

        assert_eq!(false, filter.apply(&payload));
    }

}
