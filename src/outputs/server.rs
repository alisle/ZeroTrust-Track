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

enum MessageType {
    Open(String),
    Close(String),
}

pub struct Server {
    tx: Sender<MessageType>,
}

fn send(open_url : &str, close_url: &str, message : MessageType) {
    let (url, connection) = match message {
        MessageType::Open(connection) => (open_url, connection),
        MessageType::Close(connection) => (close_url, connection),
    };

    let res = reqwest::Client::new()
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(connection)
        .send();

    match res {
        Err(err) => error!("unable to send to server: {}", err),
        Ok(mut res) => {
            match res.status() {
                StatusCode::OK => info!("successfully inserted connection"),
                _ => match res.text() {
                    Err(err) => error!("failed to insert to server: {}", err),
                    Ok(body) => error!("failed to insert to server: {}", body),
                },
            }
        }
    }
}


impl Server {
    pub fn new(url: &str) -> Result<Server, String> {
        let open_url = format!("{}/connections/open", url);
        let close_url = format!("{}/connections/close", url);

        let (tx, rx) = channel();

        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(message) => send(&open_url, &close_url, message),
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
