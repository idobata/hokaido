use std;
use std::collections::HashMap;
use std::{fmt, io, result};
use std::fmt::Display;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::{self, channel, Receiver, Sender};

use message;

#[derive(Debug)]
pub enum Error {
    NotificationSend(mpsc::SendError<Option<message::Notification>>),
    Message(message::Error),
    Io(io::Error),
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "Server error"
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::NotificationSend(ref err) => Some(err),
            Error::Message(ref err) => Some(err),
            Error::Io(ref err) => Some(err),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::error::Error::description(self).fmt(f)
    }
}

impl From<message::Error> for Error {
    fn from(err: message::Error) -> Error {
        Error::Message(err)
    }
}

impl From<mpsc::SendError<Option<message::Notification>>> for Error {
    fn from(err: mpsc::SendError<Option<message::Notification>>) -> Error {
        Error::NotificationSend(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

pub type Result<T> = result::Result<T, Error>;

pub fn execute(host: String, port: i32) -> Result<()> {
    let listener = try!(TcpListener::bind(&format!("{}:{}", host, port)[..]));
    let mut channels = Channels { channels: HashMap::new() };

    info!("Starting hokaido server on {}", listener.local_addr().unwrap());

    for stream in listener.incoming() {
        let _ = handle_client(stream, &mut channels);
    }

    Ok(())
}

fn handle_client(stream: result::Result<TcpStream, io::Error>, channels: &mut Channels) -> Result<()> {
    let stream = try!(stream);

    info!("{} Connected", stream.peer_addr().unwrap());
    stream.set_write_timeout(Some(Duration::new(10, 0))).unwrap();

    let request = try!(message::JoinRequest::receive(&stream));

    match request {
        message::JoinRequest::Broadcast(ch) => {
            let _ = BroadcastHandler::spawn(&stream, channels.fetch(&ch));
        }
        message::JoinRequest::Watch(ch) => {
            let _ = WatchHandler::spawn(&stream, channels.fetch(&ch));
        }
    }

    Ok(())
}

struct Channels {
    channels: HashMap<String, Arc<Mutex<Channel>>>,
}

struct Channel {
    broadcaster: Option<TcpStream>,
    watchers: Vec<TcpStream>,
}

struct BroadcastHandler {
    stream: TcpStream,
    channel: Arc<Mutex<Channel>>,
}

struct WatchHandler {
    stream: TcpStream,
    channel: Arc<Mutex<Channel>>,
}

impl Channels {
    fn fetch<K: AsRef<str>>(&mut self, key: &K) -> &Arc<Mutex<Channel>> {
        let ch = key.as_ref();

        if self.channels.contains_key(&ch[..]) {
            self.channels.get(&ch[..]).unwrap()
        } else {
            let channel = Channel {
                broadcaster: None,
                watchers: Vec::new(),
            };

            info!("Creating new channel: {}", ch);
            self.channels.insert(ch.to_owned(), Arc::new(Mutex::new(channel)));

            self.fetch(&ch)
        }
    }
}

impl Channel {
    fn takeover(&mut self, stream: TcpStream) -> Result<()> {
        match self.broadcaster.as_mut() {
            Some(mut former) => {
                match former.peer_addr() {
                    Ok(addr) => {
                        info!("{} Takeover from {}", stream.peer_addr().unwrap(), addr);

                        try!(message::Notification::Closed("Broadcaster has changed".to_owned()).send(&mut former));
                        try!(former.shutdown(Shutdown::Both));
                    }
                    Err(_) => () // former has gone
                }
            }
            None => () // no broadcaster yet
        }

        self.broadcaster = Some(stream);

        Ok(())
    }
}

impl BroadcastHandler {
    fn spawn(stream: &TcpStream, channel: &Arc<Mutex<Channel>>) -> Result<()> {
        info!("{} Broadcast", stream.peer_addr().unwrap());

        let mut handler = BroadcastHandler {
            stream: try!(stream.try_clone()),
            channel: channel.clone(),
        };


        match channel.lock() {
            Ok(mut ch) => {
                let _ = ch.takeover(try!(stream.try_clone()));
            }
            Err(_) => ()
        }

        thread::spawn(move || {
            handler.process().unwrap_or_else(|e| {
                warn!("{}", e);
            });

            info!("{} Shutting down", handler.stream.peer_addr().unwrap());
            let _ = handler.stream.shutdown(Shutdown::Both);
        });

        Ok(())
    }

    fn process(&mut self) -> Result<()> {
        try!(message::JoinResponse::Success.send(&mut self.stream));

        let (sender, receiver) = channel();

        try!(self.spawn_relay(sender.clone()));
        try!(self.broadcast(receiver));

        Ok(())
    }

    fn spawn_relay(&self, sender: Sender<Option<message::Notification>>) -> Result<()> {
        let stream = try!(self.stream.try_clone());

        thread::spawn(move || -> Result<()> {
            while let Ok(notification) = message::Notification::receive(&stream) {
                match notification {
                    message::Notification::Output(data)   => {
                        sender.send(Some(message::Notification::Output(data))).unwrap_or_else(|e| warn!("{}", e));
                    }
                    _ => break,
                }
            };

            info!("{} Relay stopped", stream.peer_addr().unwrap());

            try!(sender.send(None));

            Ok(())
        });

        Ok(())
    }

    fn broadcast(&self, receiver: Receiver<Option<message::Notification>>) -> Result<()> {
        for message in receiver {
            match message {
                Some(notification) => {
                    let mut channel = self.channel.lock().unwrap();

                    for watcher in &mut channel.watchers {
                        let _ = notification.send(watcher);
                    }
                },
                None => break
            }
        }

        Ok(())
    }
}

impl WatchHandler {
    fn spawn(stream: &TcpStream, channel: &Arc<Mutex<Channel>>) -> Result<()> {
        info!("{} Watch", stream.peer_addr().unwrap());

        let mut handler = WatchHandler {
            stream: try!(stream.try_clone()),
            channel: channel.clone(),
        };

        thread::spawn(move || {
            handler.process().unwrap_or_else(|e| {
                warn!("{}", e);
            });
        });

        Ok(())
    }

    fn process(&mut self) -> Result<()> {
        try!(message::JoinResponse::Success.send(&mut self.stream));

        let mut channel = self.channel.lock().unwrap();

        channel.watchers.push(try!(self.stream.try_clone()));

        match channel.broadcaster.as_mut() {
            Some(mut broadcaster) => message::Notification::WatcherJoined("".to_owned())
                                         .send(&mut broadcaster)
                                         .unwrap(),
            None => ()
        }

        Ok(())
    }
}
