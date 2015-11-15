use std::{self, fmt, result};
use std::io::{self, Write};
use std::net::TcpStream;

use message;

#[derive(Debug)]
pub enum Error {
    Message(message::Error),
    Io(io::Error),
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "Watch error"
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::Message(ref err) => Some(err),
            Error::Io(ref err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::error::Error::description(self).fmt(f)
    }
}

impl From<message::Error> for Error {
    fn from(err: message::Error) -> Error {
        Error::Message(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

pub type Result<T> = result::Result<T, Error>;

pub fn execute(host: String, port: i32, channel_name: String) -> Result<()> {
    let mut stream = try!(TcpStream::connect(&format!("{}:{}", host, port)[..]));

    let request = message::JoinRequest::Watch(channel_name.clone());
    try!(request.send(&mut stream));
    try!(message::JoinResponse::receive(&stream));

    NotificationHandler::execute(&stream)
}

struct NotificationHandler {
    stream: TcpStream,
}

impl NotificationHandler {
    fn execute(stream: &TcpStream) -> Result<()> {
        let mut handler = NotificationHandler { stream: stream.try_clone().unwrap() };

        handler.process()
    }

    fn process(&mut self) -> Result<()> {
        loop {
            let notification = try!(message::Notification::receive(&self.stream));

            match notification {
                message::Notification::Output(data) => self.handle_output(data),
                message::Notification::Closed(reason) => self.handle_closed(reason),
                _ => (),
            }
        }
    }

    fn handle_output(&self, data: String) {
        print!("{}", data);

        let _ = io::stdout().flush();
    }

    fn handle_closed(&self, reason: String) {
        println!("Connection closed: {}", reason);

        std::process::exit(0);
    }
}
