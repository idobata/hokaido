use std::io::{self, Write};
use std::net::TcpStream;

use message;

pub fn execute(host: String, port: i32, channel_name: String) {
    let mut stream = TcpStream::connect(&format!("{}:{}", host, port)[..]).unwrap();

    let request = message::JoinRequest::Watch(channel_name.clone());
    request.send(&mut stream).unwrap();
    message::JoinResponse::receive(&stream).unwrap();

    NotificationHandler::execute(&stream)
}

struct NotificationHandler {
    stream: TcpStream,
}

impl NotificationHandler {
    fn execute(stream: &TcpStream) {
        let mut handler = NotificationHandler { stream: stream.try_clone().unwrap() };

        handler.process();
    }

    fn process(&mut self) {
        loop {
            let notification = message::Notification::receive(&self.stream).unwrap();

            match notification {
                message::Notification::Output(data) => self.handle_output(data),
                message::Notification::Closed(reason) => self.handle_closed(reason),
                _ => (),
            }
        }
    }

    fn handle_output(&self, data: String) {
        print!("{}", data);
        io::stdout().flush().unwrap();
    }

    fn handle_closed(&self, reason: String) {
        println!("Connection closed: {}", reason);

        ::std::process::exit(0);
    }
}
