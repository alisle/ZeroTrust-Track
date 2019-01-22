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

use std::sync::mpsc::Sender;
use std::sync::mpsc::channel;
use std::thread;
use outputs::{ Output };
use reqwest;
use reqwest::{ StatusCode };
use reqwest::header::{ CONTENT_TYPE };
use uuid::Uuid;
use serde_json;
use ipnetwork::IpNetwork;
use std::net::Ipv4Addr;



enum MessageType {
    Open(String),
    Close(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenMessage {
    uuid: Option<Uuid>,
    name: Option<String>,
    interfaces : Vec<Ipv4Addr>
}

#[derive(Debug, Serialize, Deserialize)]
struct InterfaceMessage {
    interfaces: Vec<Ipv4Addr>
}

pub struct Server {
    tx: Sender<MessageType>,
    timer: timer::Timer,
    interface_update_guard : Option<timer::Guard>,
}

fn post(payload: &str, url: &str) -> Result<(), String> {
    let payload = String::from(payload);
    let res = reqwest::Client::new()
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(payload)
        .send();

    match res {
        Err(err) => Err(format!("unable to send to server: {}", err)),
        Ok(mut res) => {
            match res.status() {
                StatusCode::OK => Ok(()),
                    _ => match res.text() {
                    Err(err) => Err(format!("failed to insert to server: {}", err)),
                    Ok(body) => Err(format!("failed to insert to server: {}", body)),
                },
            }
        }
    }
}

fn send_connection(open_url : &str, close_url: &str, message : MessageType) {
    let (url, connection) = match message {
        MessageType::Open(connection) => (open_url, connection),
        MessageType::Close(connection) => (close_url, connection),
    };

    match post(&connection, url) {
        Err(err) => error!("{}", err),
        Ok(()) => info!("successfully sent connection to zerotrust server"),
    };
}

fn open_connection(url: &str, open_message: OpenMessage) -> Result<(), String>{
    let open_message = match serde_json::to_string(&open_message) {
        Ok(x) => x,
        Err(_err) => return Err(String::from("unable to serialize the open_message!")),
    };

    info!("marking agent online to URL: {} with payload: \"{}\"", url, open_message);
    post(&open_message, url)
}


fn get_interfaces() ->  Vec<Ipv4Addr>{
    let mut interfaces : Vec<Ipv4Addr> = Vec::new();

    for interface in pnet::datalink::interfaces() {
        for address in interface.ips {
            if let IpNetwork::V4(network) = address {
                let ip = network.ip();
                if !ip.is_loopback() {
                    interfaces.push(ip);
                }
            }
        }
    }
    interfaces
}


fn send_interfaces(url: &str, interfaces_message: InterfaceMessage) -> Result<(), String> {
    let interfaces_message = match serde_json::to_string(&interfaces_message) {
        Ok(x) => x,
        Err(_err) => return Err(String::from("unable to serialize the interface_mesage!")),
    };

    info!("sending interface information to URL: {} with payload: \"{}\"", url, interfaces_message);
    post(&interfaces_message, url)
}


fn create_interface_scheduled_call(timer: &timer::Timer, minutes : i64, url: &str) -> timer::Guard  {
    let url : String = String::from(url);
    debug!("setting timer to {}", minutes);
    timer.schedule_repeating(chrono::Duration::minutes(minutes), move || {
        let interfaces = get_interfaces();
        debug!("getting interfaces");
        debug!("found IPs: {:?}", interfaces);
        let interface_message =  InterfaceMessage {
            interfaces
        };

        match send_interfaces(&url, interface_message) {
            Ok(()) => info!("successfully send interface information"),
            Err(_err) => error!("unable to update the interface information")
        };
    })
}


impl Server {
    pub fn new(name: &Option<String>, uuid: &Option<Uuid>, url: &str) -> Result<Server, String> {
        let timer : timer::Timer = timer::Timer::new();
        let open_message =  OpenMessage {
            name: name.clone(),
            uuid: uuid.clone(),
            interfaces: get_interfaces(),
        };


        let open_url = format!("{}/connections/open", url);
        let close_url = format!("{}/connections/close", url);
        let open_connection_url = format!("{}/agents/online", url);

        match open_connection(&open_connection_url, open_message) {
            Ok(()) => info!("successfully opened agent on server"),
            Err(err) => return Err(err),
        };

        let interface_update_guard = match uuid {
            Some(uuid) => {
                debug!("creating callback guard");
                let interface_url = format!("{}/agents/{}/interfaces", url, uuid);
                Some(create_interface_scheduled_call(&timer, 30, &interface_url))
            },
            None => {
                warn!("unable to send interface details as uuid isn't set");
                None
            }
        };


        let (tx, rx) = channel();

        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(message) => send_connection(&open_url, &close_url, message),
                    Err(err) => {
                        error!("closing thread: {}", err);
                        break;
                    }
                }
            }
        });

        Ok(Server {
            tx,
            timer,
            interface_update_guard
        })
    }

}

impl Output for Server {
    fn process_open_connection(&mut self, message: &str) {
        let _ = self.tx.send(MessageType::Open(message.to_string()));
    }

    fn process_close_connection(&mut self, message: &str) {
        let _ = self.tx.send(MessageType::Close(message.to_string()));
    }

}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_interfaces() {
        get_interfaces();
    }
}
