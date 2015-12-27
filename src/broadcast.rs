use std::{env, error, fmt, io, process, result, thread};
use std::net::TcpStream;
use std::sync::mpsc::{channel, Sender};

use pty;
use pty_shell::{self, winsize, PtyProxy};
use libc;

use message;

#[derive(Debug)]
pub enum Error {
    Message(message::Error),
    Pty(pty::Error),
    PtyShell(pty_shell::Error),
    Io(io::Error),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "Broadcast error"
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Message(ref err) => Some(err),
            Error::Pty(ref err) => Some(err),
            Error::PtyShell(ref err) => Some(err),
            Error::Io(ref err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        error::Error::description(self).fmt(f)
    }
}

impl From<message::Error> for Error {
    fn from(err: message::Error) -> Error {
        Error::Message(err)
    }
}

impl From<pty::Error> for Error {
    fn from(err: pty::Error) -> Error {
        Error::Pty(err)
    }
}

impl From<pty_shell::Error> for Error {
    fn from(err: pty_shell::Error) -> Error {
        Error::PtyShell(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

pub type Result<T> = result::Result<T, Error>;

struct NotificationHandler {
    stream: TcpStream,
    sender: Sender<Option<message::Notification>>,
}

struct ShellHandler {
    channel: Sender<Option<message::Notification>>,
}

impl pty_shell::PtyHandler for ShellHandler {
    fn output(&mut self, data: &[u8]) {
        let string = String::from_utf8_lossy(&data[..]).into_owned();
        let _ = self.channel.send(Some(message::Notification::Output(string)));
    }

    fn resize(&mut self, size: &winsize::Winsize) {
        let _ = self.channel.send(Some(build_winsize_notification(size)));
    }

    fn shutdown(&mut self) {
        let _ = self.channel.send(None);
    }
}

pub fn execute(host: String, port: i32, channel_name: String) -> Result<()> {
    let mut stream = try!(TcpStream::connect(&format!("{}:{}", host, port)[..]));
    let (sender, receiver) = channel();

    try!(message::JoinRequest::Broadcast(channel_name.clone()).send(&mut stream));
    try!(message::JoinResponse::receive(&stream));

    NotificationHandler::spawn(&stream, &sender);

    let child = try!(pty::fork());
    try!(child.exec(env::var("SHELL").unwrap_or("bash".to_owned())));
    try!(child.proxy(ShellHandler { channel: sender.clone() }));

    let winsize = try!(winsize::from_fd(libc::STDIN_FILENO));
    let _ = sender.send(Some(build_winsize_notification(&winsize)));

    for message in receiver {
        match message {
            Some(notification) => try!(notification.send(&mut stream)),
            None => break,
        }
    }

    try!(child.wait());

    Ok(())
}

impl NotificationHandler {
    fn spawn(stream: &TcpStream, sender: &Sender<Option<message::Notification>>) {
        let mut handler = NotificationHandler {
            stream: stream.try_clone().unwrap(),
            sender: sender.clone(),
        };

        thread::spawn(move || {
            handler.process().unwrap_or_else(|e| {
                println!("{:?}", e);
                process::exit(1);
            });
        });
    }

    fn process(&mut self) -> Result<()> {
        loop {
            let notification = try!(message::Notification::receive(&self.stream));

            match notification {
                message::Notification::Closed(reason) => {
                    println!("Broadcast has stopped: {}", reason);
                    break;
                }
                message::Notification::WatcherJoined(_) => {
                    let winsize = try!(winsize::from_fd(libc::STDIN_FILENO));
                    let _ = self.sender.send(Some(build_winsize_notification(&winsize)));
                }
                _ => (),
            }
        }

        let _ = self.sender.send(None);

        Ok(())
    }
}

fn build_winsize_notification(size: &winsize::Winsize) -> message::Notification {
    message::Notification::Output(format!("\x1b[8;{};{}t", size.ws_row, size.ws_col))
}
