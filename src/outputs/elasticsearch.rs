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

pub struct Elasticsearch {
    tx : Sender<String>,
}


impl Elasticsearch {
    pub fn new(url: &str) -> Result<Elasticsearch, String> {
        let url = format!("{}/_doc", url);

        let (tx, rx) = channel();

        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(message) => {
                        info!("sending payload to ES: {}", &message);
                        let res = reqwest::Client::new()
                            .post(&url)
                            .header(CONTENT_TYPE, "application/json")
                            .body(message)
                            .send();

                        match res {
                            Err(err) => error!("unable to send to ES: {}", err),
                            Ok(mut res) => {
                                match res.status() {
                                     StatusCode::CREATED => info!("successfully inserted into ES"),
                                     _ => match res.text() {
                                             Err(err) => error!("failed to insert to ES: {}", err),
                                             Ok(body) => error!("failed to insert to ES: {}", body)
                                     },
                                }
                            }
                        };

                    },
                    Err(err) => {
                        error!("closing thread: {}", err);
                        break;
                    }
                }
            }
        });

        Ok(Elasticsearch {
            tx
        })
    }
}

impl Output for Elasticsearch {
    fn process(&mut self, message: &str) {
        let _ = self.tx.send(message.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_url() {
        let elasticsearch = Elasticsearch::new("http://127.0.0.1:9200");
        assert!(!elasticsearch.is_err());
    }
}
