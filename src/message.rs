use std;
use std::fmt;
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
}

#[derive(Debug)]
pub enum Error {
    EncodeError(msgpack::encode::Error),
    DecodeError(msgpack::decode::Error),
    UnknownMessage,
}

impl JoinRequest {
    pub fn receive(stream: &TcpStream) -> Result<JoinRequest, Error> {
        let mut decoder = Decoder::new(stream);

        match Decodable::decode(&mut decoder) {
            Ok(message) => {
                let (_, _, role, channel_name): (u8, u8, String, String) = message;

                match role.as_ref() {
                    "broadcast" => Ok(JoinRequest::Broadcast(channel_name)),
                    "watch"     => Ok(JoinRequest::Watch(channel_name)),
                    _           => Err(Error::UnknownMessage)
                }

            },
            Err(e) => Err(Error::from(e))
        }
    }

    pub fn send(&self, stream: &mut TcpStream) -> Result<(), Error> {
        let mut encoder = Encoder::new(&mut *stream);

        self.payload().encode(&mut encoder).or_else(|e| Err(Error::from(e)))
    }

    fn payload(&self) -> (u8, u8, &str, &String) {
        let header = 0u8;
        let id     = 0u8;

        match *self {
            JoinRequest::Broadcast(ref string) => (header, id, "broadcast", string),
            JoinRequest::Watch(ref string)     => (header, id, "watch", string),
        }
    }
}

impl JoinResponse {
    pub fn receive(stream: &TcpStream) -> Result<JoinResponse, Error> {
        let mut decoder = Decoder::new(stream);

        match Decodable::decode(&mut decoder) {
            Ok(message) => {
                let (_, _, _, result): (u8, u8, String, bool) = message;

                match result {
                    true  => Ok(JoinResponse::Success),
                    false => Ok(JoinResponse::Failure),
                }

            },
            Err(e) => Err(Error::from(e))
        }
    }

    pub fn send(&self, stream: &mut TcpStream) -> Result<(), Error> {
        let mut encoder = Encoder::new(&mut *stream);

        self.payload().encode(&mut encoder).or_else(|e| Err(Error::from(e)))
    }

    fn payload(&self) -> (u8, u8, &str, bool) {
        let header = 0u8;
        let id     = 0u8;

        match *self {
            JoinResponse::Success => (header, id, "", true),
            JoinResponse::Failure => (header, id, "", false),
        }
    }
}

impl Notification {
    pub fn receive(stream: &TcpStream) -> Result<Notification, Error> {
        let mut decoder = Decoder::new(stream);

        match Decodable::decode(&mut decoder) {
            Ok(message) => {
                let (_, topic, data): (u8, String, String) = message;

                match topic.as_ref() {
                    "out"    => Ok(Notification::Output(data)),
                    "closed" => Ok(Notification::Closed(data)),
                    _        => Err(Error::UnknownMessage)
                }

            },
            Err(e) => Err(Error::from(e))
        }
    }

    pub fn send(&self, stream: &mut TcpStream) -> Result<(), Error> {
        let mut encoder = Encoder::new(&mut *stream);

        self.payload().encode(&mut encoder).or_else(|e| Err(Error::from(e)))
    }

    fn payload(&self) -> (u8, &str, &String) {
        let header = 2u8;

        match *self {
            Notification::Output(ref string) => (header, "out", string),
            Notification::Closed(ref string) => (header, "closed", string),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "Processing message failed"
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::EncodeError(ref err) => Some(err),
            Error::DecodeError(ref err) => Some(err),
            _ => None
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
        Error::EncodeError(err)
    }
}

impl From<msgpack::decode::Error> for Error {
    fn from(err: msgpack::decode::Error) -> Error {
        Error::DecodeError(err)
    }
}
