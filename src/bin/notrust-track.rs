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

extern crate clap;
extern crate notrust_track;
extern crate simple_logger;

#[macro_use]
extern crate log;

use clap::{Arg, App};
use notrust_track::{NoTrack};

fn main() {
    simple_logger::init().unwrap();

    let matches = App::new("NoTrust Tracker")
        .version("1.0")
        .author("Alex Lisle <alex.lisle@gmail.com>")
        .about("Tracks all incoming and outgoing TCP/UDP Connections and their corresponding processes and users who launched them")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Defines a custom config file")
            .takes_value(true)
            .required(false)
        ).get_matches();

    let config = matches.value_of("config").unwrap_or("/etc/notrust/config.yaml");
    info!("loading config: {}", config);

    let mut app = match NoTrack::from_file(&config) {
        Ok(app) => app,
        Err(err) => {
            error!("{}", err);
            return;
        },
    };

    if let Err(err) = app.run() {
        error!("{}", err);
    }

}
