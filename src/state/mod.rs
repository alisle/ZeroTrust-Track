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

use std::collections::HashMap;
use parser::{ Payload, CloseConnection };
use uuid::Uuid;

pub struct State {
    connections: HashMap<i64, Uuid>
}

impl State {
    pub fn new() -> Result<State, ()> {
        let state = State {
            connections: HashMap::new()
        };

        Ok(state)
    }

    pub fn transform(&mut self, payload: Payload) -> Payload {
        match payload {
            Payload::Open(connection )=> {
                self.connections.insert(connection.hash, connection.uuid.clone());
                return Payload::Open(connection);
            },
            Payload::Close(connection) =>  {
                match self.connections.remove(&connection.hash) {
                   Some(uuid) =>  return Payload::Close(CloseConnection { uuid: Some(uuid), .. connection }),
                   None => return Payload::Close(connection),
               }
           }
       }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use parser::{ Payload, OpenConnection, CloseConnection };
    use enums::{ Protocol };
    use std::net::Ipv4Addr;
    use parser::{ Program, generate_hash };
    use chrono::prelude::*;
    use uuid::Uuid;

    fn default_close_payload() -> Payload {
        Payload::Close(CloseConnection {
            hash: generate_hash(
                &Protocol::TCP.to_string(),
                &Ipv4Addr::new(127, 0, 0, 1),
                &22,
                &Ipv4Addr::new(127, 0, 0, 1),
                &22
            ) as i64,
            uuid: None,
            agent: Uuid::new_v4(),            
            timestamp: Utc::now().to_rfc3339(),
            protocol: Protocol::TCP,
            source_port : 22,
            source: Ipv4Addr::new(127, 0, 0, 1),
            destination_port : 22,
            destination : Ipv4Addr::new(127, 0, 0, 1),
        })
    }

    fn default_open_payload(
        source_port : u16,
        destination_port : u16,
        program_details: Option<Program>
    ) -> Payload {
        Payload::Open(OpenConnection {
            hash: generate_hash(
                &Protocol::TCP.to_string(),
                &Ipv4Addr::new(127, 0, 0, 1),
                &22,
                &Ipv4Addr::new(127, 0, 0, 1),
                &22
            ) as i64,
            uuid: Uuid::new_v4(),
            agent: Uuid::new_v4(),
            timestamp: Utc::now().to_rfc3339(),
            protocol: Protocol::TCP,
            source_port : source_port,
            source: Ipv4Addr::new(127, 0, 0, 1),
            destination_port : destination_port,
            destination : Ipv4Addr::new(127, 0, 0, 1),
            username : String::from("hello"),
            uid: 10,
            program_details : program_details,
        })
    }

    #[test]
    fn test_no_state() {
        let mut state = State::new().unwrap();
        let close_payload = default_close_payload();
        if let Payload::Close(ref close_connection) = close_payload {
            assert_eq!(true, close_connection.uuid.is_none());
        } else {
            assert_eq!(true, false);
        }

        let close_payload = state.transform(close_payload);

        if let Payload::Close(ref close_connection) = close_payload {
            assert_eq!(true, close_connection.uuid.is_none());
        } else {
            assert_eq!(true, false);
        }

    }
    #[test]
    fn test_added_state() {
        let mut state = State::new().unwrap();
        let open_payload = default_open_payload(22, 22, None);
        let close_payload = default_close_payload();
        if let Payload::Close(ref close_connection) = close_payload {
            assert_eq!(true, close_connection.uuid.is_none());
        } else {
            assert_eq!(true, false);
        }

        let open_payload = state.transform(open_payload);
        let close_payload = state.transform(close_payload);

        if let Payload::Close(close_connection) = close_payload {
            if let Payload::Open(open_connection) = open_payload {
                assert_eq!(false, close_connection.uuid.is_none());
                assert_eq!(open_connection.uuid, close_connection.uuid.unwrap());
            } else {
                assert_eq!(true, false);
            }
        } else {
            assert_eq!(true, false);
        }
    }

}
