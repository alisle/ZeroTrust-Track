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

use std::mem::size_of;
use std::net::Ipv4Addr;
use std::io;

extern crate libc;
extern crate crslmnl as mnl;
extern crate log;

use mnl::linux::netlink as netlink;
use mnl::linux::netfilter::nfnetlink_conntrack as conntrack;
use mnl::linux::netfilter::nfnetlink as nfnetlink;
use std::sync::mpsc::Sender;

use enums::{Protocol, State};



#[derive(Debug)]
pub enum ProtoDetails {
    ICMP {
        icmp_id : u16,
        icmp_type : u8,
        icmp_code : u8
    },
    IP {
        protocol : Protocol,
        source_port : u16,
        destination_port : u16
    },
    NotSupported
}

#[derive(Debug)]
pub struct ConnectionDetails {
    pub source: Ipv4Addr,
    pub destination : Ipv4Addr,
    pub protocol : ProtoDetails
}



#[derive(Debug)]
pub struct Connection {
    pub state : State,
    pub details : ConnectionDetails
}

pub struct Conntrack<'a> {
    socket : &'a mut mnl::Socket,
}

impl<'a> Conntrack<'a> {
    pub fn new() -> Result<Conntrack<'a>, io::Error> {
        let nl =  mnl::Socket::open(netlink::Family::NETFILTER)?;
        nl.bind(conntrack::NF_NETLINK_CONNTRACK_NEW, mnl::SOCKET_AUTOPID)?;  //| conntrack::NF_NETLINK_CONNTRACK_DESTROY, mnl::SOCKET_AUTOPID)?;

        Ok(Conntrack {
            socket: nl,
        })
    }

    pub fn start(&mut self, tx: &mut Sender<Connection>) {
        let mut buf = vec![0u8; mnl::SOCKET_BUFFER_SIZE()];
        loop {
            let recv = self.socket.recvfrom(&mut buf)
                .unwrap_or_else(|errno| panic!("failed to recieve from conntrack! {}", errno));
            trace!("received connection update");

            mnl::cb_run(&buf[0..recv], 0, 0, Some(process_data_callback), tx)
                .unwrap_or_else(|errno| panic!("failed to invoke callback! {}", errno));
        }
    }

}


//***********************************************************************************************************************************************
// Call Backs
//***********************************************************************************************************************************************
#[allow(dead_code)]
fn process_proto_callback<'a>(attr: &'a mnl::Attr, tb: &mut[Option<&'a mnl::Attr>]) -> mnl::CbRet {
    if let Err(_) = attr.type_valid(conntrack::CTA_PROTO_MAX) {
        return mnl::CbRet::OK;
    }

    let attribute_type = attr.atype();
    match attribute_type {
        n if (n == conntrack::CtattrL4proto::NUM as u16 ||
            n == conntrack::CtattrL4proto::ICMP_TYPE as u16 ||
            n == conntrack::CtattrL4proto::ICMP_CODE as u16) => {
            if let Err(errno) = attr.validate(mnl::AttrDataType::U8) {
                // Need to do error handling
                error!("unable to validate protocol {}", errno);
                return mnl::CbRet::ERROR;
            }
        },
        n if (n == conntrack::CtattrL4proto::SRC_PORT as u16 ||
            n == conntrack::CtattrL4proto::DST_PORT as u16 ||
            n == conntrack::CtattrL4proto::ICMP_ID as u16) => {
            if let Err(errno) = attr.validate(mnl::AttrDataType::U16) {
                error!("unable to validate protocol {}", errno);
                return mnl::CbRet::ERROR;
            }
        },
        _ => {},
    }

    tb[attribute_type as usize] = Some(attr);
    mnl::CbRet::OK
}

#[allow(dead_code)]
fn process_ip_callback<'a>(attr: &'a mnl::Attr, tb: &mut [Option<&'a mnl::Attr>]) -> mnl::CbRet {
    if let Err(_) = attr.type_valid(conntrack::CTA_IP_MAX) {
        return mnl::CbRet::OK
    }

    let attribute_type = attr.atype();
    match attribute_type {
        n if (n == conntrack::CtattrIp::V4_SRC as u16 ||
            n == conntrack::CtattrIp::V4_DST as u16) => {
            if let Err(errno) = attr.validate(mnl::AttrDataType::U32) {
                error!("unable to validate ip {}", errno);
                return mnl::CbRet::ERROR;
            }
        },
        _ => {},
    }

    tb[attribute_type as usize] = Some(attr);
    mnl::CbRet::OK
}

#[allow(dead_code)]
fn process_tuple_callback<'a>(attr: &'a mnl::Attr, tb: &mut [Option<&'a mnl::Attr>]) -> mnl::CbRet {
    if let Err(_) = attr.type_valid(conntrack::CTA_TUPLE_MAX) {
        return mnl::CbRet::OK;
    }

    let attribute_type = attr.atype();
    match attribute_type {
        n if n == conntrack::CtattrTuple::IP as u16 => {
            if let Err(errno) = attr.validate(mnl::AttrDataType::NESTED) {
                error!("unable to validate tuple {}", errno);
                return mnl::CbRet::ERROR;
            }
        },
        n if n == conntrack::CtattrTuple::PROTO as u16 => {
            if let Err(errno) = attr.validate(mnl::AttrDataType::NESTED) {
                error!("unable to validate tuple {}", errno);
                return mnl::CbRet::ERROR
            }
        },
        _ => {},
    }

    tb[attribute_type as usize] = Some(attr);
    mnl::CbRet::OK
}


#[allow(dead_code)]
fn process_attributes_callback<'a>(attr: &'a mnl::Attr, buf: &mut [Option<&'a mnl::Attr>]) -> mnl::CbRet {
    if let Err(_) = attr.type_valid(conntrack::CTA_MAX as u16) {
        return mnl::CbRet::OK;
    }

    let attribute_type = attr.atype();
    match attribute_type {
        n if n == conntrack::CtattrType::TUPLE_ORIG as u16 => {
            if let Err(errno) = attr.validate(mnl::AttrDataType::NESTED) {
                error!("unable to validate attributes {}", errno);
                return mnl::CbRet::ERROR;
            }
        },
        n if (n == conntrack::CtattrType::TIMEOUT as u16 ||
            n == conntrack::CtattrType::MARK as u16 ||
            n == conntrack::CtattrType::SECMARK as u16) => {
            if let Err(errno) = attr.validate(mnl::AttrDataType::U32) {
                error!("unable to validate attributes {}", errno);
                return mnl::CbRet::ERROR;
            }
        },
        _ => {},
    }

    buf[attribute_type as usize] = Some(attr);
    mnl::CbRet::OK
}



#[allow(dead_code)]
fn process_data_callback(message : mnl::Nlmsg, sender: &mut Sender<Connection>) -> mnl::CbRet {
    let mut buf: [Option<&mnl::Attr>; conntrack::CTA_MAX as usize + 1] = [None; conntrack::CTA_MAX as usize + 1];


    let state : State = match *message.nlmsg_type & 0xFF {
        n if n == conntrack::CtnlMsgTypes::NEW as u16 => {
            if *message.nlmsg_flags & (netlink::NLM_F_CREATE) != 0 {
                State::New
            } else {
                State::Unknown
            }
        },
        n if n == conntrack::CtnlMsgTypes::DELETE as u16 => {
            State::Destroy
        },
        _ => { State::Unknown }
    };

    trace!("state: {:?}", state);

    let _ = message.parse(size_of::<nfnetlink::Nfgenmsg>(), process_attributes_callback, &mut buf);
    let details = extract_tuple(buf[conntrack::CtattrType::TUPLE_ORIG as usize].unwrap());
    let connection = Connection {
        state,
        details
    };

    debug!("sending {:?} over channel", connection);
    if let Err(x) = sender.send(connection) {
        // Handle error.
        error!("unable to send connection details {:?}", x);
    }

    mnl::CbRet::OK
}

//***********************************************************************************************************************************************
// Extractions
//***********************************************************************************************************************************************
#[allow(dead_code)]
fn extract_ip(nest: &mnl::Attr) -> (Option<Ipv4Addr>, Option<Ipv4Addr>){
    let mut buf: [Option<&mnl::Attr>; conntrack::CTA_IP_MAX as usize + 1] = [None; conntrack::CTA_IP_MAX as usize + 1];
    let _ = nest.parse_nested(process_ip_callback, &mut buf);

    let source = match buf[conntrack::CtattrIp::V4_SRC as usize] {
        None => None,
        Some(attribute) => Some(attribute.payload::<Ipv4Addr>().clone())
    };

    let destination = match buf[conntrack::CtattrIp::V4_DST as usize] {
        None => None,
        Some(attribute) => Some(attribute.payload::<Ipv4Addr>().clone())
    };

    (source, destination)
}
#[allow(dead_code)]
fn extract_proto(nest: &mnl::Attr) -> ProtoDetails {
    let mut tb: [Option<&mnl::Attr>; conntrack::CTA_PROTO_MAX as usize + 1] = [None; conntrack::CTA_PROTO_MAX as usize + 1];

    let _ = nest.parse_nested(process_proto_callback, &mut tb);

    let proto = tb[conntrack::CtattrL4proto::NUM as usize].unwrap().u8();

    let source = match tb[conntrack::CtattrL4proto::SRC_PORT as usize] {
        None => None,
        Some(attribute) => Some(u16::from_be(attribute.u16()))
    };

    let destination = match tb[conntrack::CtattrL4proto::DST_PORT as usize] {
        None => None,
        Some(attribute) => Some(u16::from_be(attribute.u16()))
    };


    let icmp_id = match tb[conntrack::CtattrL4proto::ICMP_ID as usize] {
        None => None,
        Some(attribute) => Some(u16::from_be(attribute.u16()))
    };

    let icmp_type = match tb[conntrack::CtattrL4proto::ICMP_TYPE as usize] {
        None => None,
        Some(attribute) => Some(u8::from_be(attribute.u8()))
    };

    let icmp_code = match tb[conntrack::CtattrL4proto::ICMP_CODE as usize] {
        None => None,
        Some(attribute) => Some(u8::from_be(attribute.u8()))
    };

    let details = match proto {
        0x01 => ProtoDetails::ICMP { icmp_id: icmp_id.unwrap(), icmp_type: icmp_type.unwrap(), icmp_code: icmp_code.unwrap() },
        0x06 => ProtoDetails::IP{ protocol : Protocol::TCP , source_port : source.unwrap(), destination_port : destination.unwrap() },
        0x11 => ProtoDetails::IP{ protocol : Protocol::UDP , source_port : source.unwrap(), destination_port : destination.unwrap() },
        _ => ProtoDetails::NotSupported
    };

    details
}

#[allow(dead_code)]
fn extract_tuple(nest: &mnl::Attr) -> ConnectionDetails {
    let mut buf: [Option<&mnl::Attr>; conntrack::CTA_TUPLE_MAX as usize + 1] = [None; conntrack::CTA_TUPLE_MAX as usize + 1];
    let _ = nest.parse_nested(process_tuple_callback, &mut buf);

    let addresses = extract_ip(&buf[conntrack::CtattrTuple::IP as usize].unwrap());
    let protocol_details = extract_proto(buf[conntrack::CtattrTuple::PROTO as usize].unwrap());

    ConnectionDetails  {
        source : addresses.0.unwrap(),
        destination : addresses.1.unwrap(),
        protocol: protocol_details
    }
}
