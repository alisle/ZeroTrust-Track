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

use std::fmt;
use outputs::OutputsConfig;
use filters::FiltersConfig;


#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub outputs : OutputsConfig,
    pub filters : FiltersConfig,
}

#[derive(Debug, Serialize)]
pub enum Protocol {
    UDP,
    TCP,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Protocol::UDP => write!(f, "UDP"),
            Protocol::TCP => write!(f, "TCP"),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub enum State {
    New,
    Destroy,
    Unknown,
}
