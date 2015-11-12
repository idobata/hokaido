use std::collections::HashMap;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::{channel, Receiver, Sender};

use message;

pub fn execute(host: String, port: i32) {
    let listener = TcpListener::bind(&format!("{}:{}", host, port)[..]).unwrap();
    let mut channels = Channels { channels: HashMap::new() };

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let request = message::JoinRequest::receive(&stream);

        match request {
            Ok(request) => {
                match request {
                    message::JoinRequest::Broadcast(ch) =>
                        BroadcastHandler::spawn(&stream, channels.fetch(&ch)),
                    message::JoinRequest::Watch(ch) =>
                        WatchHandler::spawn(&stream, channels.fetch(&ch)),
                }
            }
            Err(e) => println!("{}", e),
        }
    }
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

            self.channels.insert(ch.to_owned(), Arc::new(Mutex::new(channel)));

            self.fetch(&ch)
        }
    }
}

impl Channel {
    fn takeover(&mut self, stream: TcpStream) {
        match self.broadcaster.as_mut() {
            Some(mut current) => {
                let _ = message::Notification::Closed("Broadcaster has changed".to_owned())
                            .send(&mut current);
                let _ = current.shutdown(Shutdown::Both);
            }
            None => (),
        }

        self.broadcaster = Some(stream);
    }
}

impl BroadcastHandler {
    fn spawn(stream: &TcpStream, channel: &Arc<Mutex<Channel>>) {
        let mut handler = BroadcastHandler {
            stream: stream.try_clone().unwrap(),
            channel: channel.clone(),
        };

        let _ = channel.lock().and_then(|mut ch| {
            ch.takeover(stream.try_clone().unwrap());
            Ok(ch)
        });

        thread::spawn(move || {
            handler.process();
        });
    }

    fn process(&mut self) {
        message::JoinResponse::Success.send(&mut self.stream).unwrap();

        let (sender, receiver) = channel();

        self.spawn_relay(sender.clone());
        self.broadcast(receiver);

        self.stream.shutdown(Shutdown::Both).unwrap();
    }

    fn spawn_relay(&self, sender: Sender<Option<message::Notification>>) {
        let stream = self.stream.try_clone().unwrap();

        thread::spawn(move || {
            loop {
                match message::Notification::receive(&stream) {
                    Ok(notification) => {
                        match notification {
                            message::Notification::Output(data)   => {
                                let _ = sender.send(Some(message::Notification::Output(data)));
                            },
                            message::Notification::Closed(reason) => {
                                let _ = sender.send(Some(message::Notification::Closed(reason)));
                            },
                            _ => break,
                        };
                    },
                    Err(_) => {
                        sender.send(None).unwrap();
                        break;
                    }
                }
            };
        });
    }

    fn broadcast(&self, receiver: Receiver<Option<message::Notification>>) {
        for message in receiver {
            match message {
                Some(notification) => {
                    for mut watcher in self.channel.lock().unwrap().watchers.iter_mut() {
                        let _ = notification.send(watcher);
                    }
                },
                None => break
            };
        }
    }
}

impl WatchHandler {
    fn spawn(stream: &TcpStream, channel: &Arc<Mutex<Channel>>) {
        let mut handler = WatchHandler {
            stream: stream.try_clone().unwrap(),
            channel: channel.clone(),
        };

        thread::spawn(move || {
            handler.process();
        });
    }

    fn process(&mut self) {
        message::JoinResponse::Success.send(&mut self.stream).unwrap();

        match self.channel.lock() {
            Ok(mut channel) => {
                channel.watchers.push(self.stream.try_clone().unwrap());

                match channel.broadcaster.as_mut() {
                    Some(mut broadcaster) => message::Notification::WatcherJoined("".to_owned())
                                                 .send(&mut broadcaster)
                                                 .unwrap(),
                    None => (),
                }
            }
            Err(e) => panic!("{}", e),
        }
    }
}
