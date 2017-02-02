use std;
use std::{fmt, result};
use std::net::TcpStream;
use rustc_serialize::{Encodable, Decodable};
use msgpack::{self, Encoder, Decoder};

pub enum JoinRequest {
    Broadcast(String),
    Watch(String),
}

pub enum JoinResponse {
    Success,
    Failure,
}

pub enum Notification {
    Output(String),
    Closed(String),
    WatcherJoined(String),
}

#[derive(Debug)]
pub enum Error {
    Encode(msgpack::encode::Error),
    Decode(msgpack::decode::Error),
    UnknownMessage,
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "Processing message failed"
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::Encode(ref err) => Some(err),
            Error::Decode(ref err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::error::Error::description(self).fmt(f)
    }
}

impl From<msgpack::encode::Error> for Error {
    fn from(err: msgpack::encode::Error) -> Error {
        Error::Encode(err)
    }
}

impl From<msgpack::decode::Error> for Error {
    fn from(err: msgpack::decode::Error) -> Error {
        Error::Decode(err)
    }
}

pub type Result<T> = result::Result<T, Error>;

impl JoinRequest {
    pub fn receive(stream: &TcpStream) -> Result<JoinRequest> {
        let (_, _, role, channel_name): (u8, u8, String, String) =
            try!(Decodable::decode(&mut Decoder::new(stream)));

        match role.as_ref() {
            "broadcast" => Ok(JoinRequest::Broadcast(channel_name)),
            "watch" => Ok(JoinRequest::Watch(channel_name)),
            _ => Err(Error::UnknownMessage),
        }
    }

    pub fn send(&self, stream: &mut TcpStream) -> Result<()> {
        let mut encoder = Encoder::new(&mut *stream);

        Ok(try!(self.payload().encode(&mut encoder)))
    }

    fn payload(&self) -> (u8, u8, &str, &String) {
        let header = 0u8;
        let id = 0u8;

        match *self {
            JoinRequest::Broadcast(ref string) => (header, id, "broadcast", string),
            JoinRequest::Watch(ref string) => (header, id, "watch", string),
        }
    }
}

impl JoinResponse {
    pub fn receive(stream: &TcpStream) -> Result<JoinResponse> {
        let (_, _, _, result): (u8, u8, String, bool) =
            try!(Decodable::decode(&mut Decoder::new(stream)));

        if result { Ok(JoinResponse::Success) } else { Ok(JoinResponse::Failure) }
    }

    pub fn send(&self, stream: &mut TcpStream) -> Result<()> {
        let mut encoder = Encoder::new(&mut *stream);

        Ok(try!(self.payload().encode(&mut encoder)))
    }

    fn payload(&self) -> (u8, u8, &str, bool) {
        let header = 0u8;
        let id = 0u8;

        match *self {
            JoinResponse::Success => (header, id, "", true),
            JoinResponse::Failure => (header, id, "", false),
        }
    }
}

impl Notification {
    pub fn receive(stream: &TcpStream) -> Result<Notification> {
        let (_, topic, data): (u8, String, String) =
            try!(Decodable::decode(&mut Decoder::new(stream)));

        match topic.as_ref() {
            "out" => Ok(Notification::Output(data)),
            "closed" => Ok(Notification::Closed(data)),
            "watcher_joined" => Ok(Notification::WatcherJoined(data)),
            _ => Err(Error::UnknownMessage),
        }
    }

    pub fn send(&self, stream: &mut TcpStream) -> Result<()> {
        let mut encoder = Encoder::new(&mut *stream);

        Ok(try!(self.payload().encode(&mut encoder)))
    }

    fn payload(&self) -> (u8, &str, &String) {
        let header = 2u8;

        match *self {
            Notification::Output(ref string) => (header, "out", string),
            Notification::Closed(ref string) => (header, "closed", string),
            Notification::WatcherJoined(ref string) => (header, "watcher_joined", string),
        }
    }
}
