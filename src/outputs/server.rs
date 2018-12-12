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

enum MessageType {
    Open(String),
    Close(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenMessage {
    uuid: Option<Uuid>,
    name: Option<String>
}

pub struct Server {
    tx: Sender<MessageType>,
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

impl Server {
    pub fn new(name: &Option<String>, uuid: &Option<Uuid>, url: &str) -> Result<Server, String> {

        let open_message =  OpenMessage {
            name: name.clone(),
            uuid: uuid.clone()
        };

        let open_connection_url = format!("{}/agents/online", url);
        match open_connection(&open_connection_url, open_message) {
            Ok(()) => info!("successfully opened agent on server"),
            Err(err) => return Err(err),
        };

        let open_url = format!("{}/connections/open", url);
        let close_url = format!("{}/connections/close", url);

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
            tx
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
