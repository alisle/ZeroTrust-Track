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

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use std::thread;

use parser::Parser;
use conn_track::Conntrack;


mod conn_track;
mod proc_chomper;
mod parser;
mod proc;


#[derive(Debug, Serialize)]
pub enum Protocol {
    UDP,
    TCP,
}


pub fn run() -> Result<(), String>{
    let mut tracker=  match Conntrack::new() {
        Ok(x) => x,
        Err(_err) => return Err(String::from("Unable to bind to conntrack, please check permissions")),
    };

    let mut parser = match Parser::new() {
        Ok(x) => x,
        Err(_err) => return Err(String::from("Unable to parse process descriptors, please check permissions")),
    };

    let (mut tx, rx) : (Sender<conn_track::Connection>, Receiver<conn_track::Connection>) = channel();

    thread::spawn(move || {
        tracker.start(&mut tx);
    });

    loop {
        let con : conn_track::Connection = rx.recv().unwrap();
        if let Some(payload) = parser.parse(con) {
            let json = serde_json::to_string(&payload).unwrap();
            println!("{}", json);
        }
    }
}
