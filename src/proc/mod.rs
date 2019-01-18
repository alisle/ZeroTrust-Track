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

use std::io;
use std::collections::HashMap;
use procfs;
use procfs::{FDTarget, Process};
use libc::pid_t;

pub struct Proc {
    map : HashMap<u32, pid_t>,
}

impl Proc {
    pub fn new() -> Result<Proc, io::Error> {
        let mut proc = Proc {
            map: HashMap::new(),
        };
        proc.update()?;

        Ok(proc)
    }

    pub fn update(&mut self) -> Result<(), io::Error> {
        let processes = procfs::all_processes();
        let mut map: HashMap<u32, pid_t> = HashMap::new();
        for process in &processes {
            if let Result::Ok(fds) = process.fd() {
                for fd in fds {
                    if let FDTarget::Socket(inode) = fd.target {
                        map.insert(inode, process.pid());
                    }
                }
            }
        }
        self.map = map;

        Ok(())
    }

    pub fn get(&mut self, inode : u32) -> Option<Process> {
        if !self.map.contains_key(&inode) {
            let _ = self.update();
        }

        match self.map.get(&inode) {
            Some(pid) => {
                match Process::new(*pid) {
                    Result::Ok(process) => Some(process),
                    _ => None
                }
            },
            None => None
        }
    }
}
