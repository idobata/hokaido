use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use message;

pub fn execute(host: String, port: i32) {
    let listener     = TcpListener::bind(&format!("{}:{}", host, port)[..]).unwrap();
    let mut channels = Channels { channels: HashMap::new() };

    for stream in listener.incoming() {
        let stream  = stream.unwrap();
        let request = message::JoinRequest::receive(&stream);

        match request {
            Ok(request) => {
                match request {
                    message::JoinRequest::Broadcast(ch) => BroadcastHandler::spawn(&stream, channels.fetch(&ch)),
                    message::JoinRequest::Watch(ch)     => WatchHandler::spawn(&stream, channels.fetch(&ch)),
                }
            },
            Err(e) => println!("{}", e)
        }
    }
}
struct Channels {
    channels: HashMap<String, Arc<Mutex<Channel>>>,
}

struct Channel {
    broadcaster: Option<TcpStream>,
    watchers:    Vec<TcpStream>,
}

struct BroadcastHandler {
    stream:  TcpStream,
    channel: Arc<Mutex<Channel>>,
}

struct WatchHandler {
    stream: TcpStream,
    channel: Arc<Mutex<Channel>>,
}

impl Channels {
    fn fetch(&mut self, ch: &String) -> &Arc<Mutex<Channel>> {
        if self.channels.contains_key(&ch[..]) {
            self.channels.get(&ch[..]).unwrap()
        } else {
            let channel = Channel { broadcaster: None, watchers: Vec::new() };

            self.channels.insert(ch.clone(), Arc::new(Mutex::new(channel)));

            self.fetch(&ch)
        }
    }
}

impl Channel {
    fn takeover(&mut self, stream: TcpStream) {
        match self.broadcaster.as_mut() {
            Some(mut current) => {
                let _ = message::Notification::Closed("Broadcaster has changed".to_string()).send(&mut current);
                let _ = current.shutdown(Shutdown::Both);
            },
            None => ()
        }

        self.broadcaster = Some(stream);
    }
}

impl BroadcastHandler {
    fn spawn(stream: &TcpStream, channel: &Arc<Mutex<Channel>>) {
        let mut handler = BroadcastHandler { stream: stream.try_clone().unwrap(), channel: channel.clone() };

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

        loop {
            let mut buf = vec![0; 128];
            let nread   = self.stream.read(&mut buf).unwrap();

            if nread == 0 { break; }

            buf.truncate(nread as usize);

            for mut watcher in self.channel.lock().unwrap().watchers.iter() {
                let _ = watcher.write(&buf);
            }
        };

        self.stream.shutdown(Shutdown::Both).unwrap();
    }
}

impl WatchHandler {
    fn spawn(stream: &TcpStream, channel: &Arc<Mutex<Channel>>) {
        let mut handler = WatchHandler { stream: stream.try_clone().unwrap(), channel: channel.clone() };

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
                    Some(mut broadcaster) => message::Notification::WatcherJoined("".to_string()).send(&mut broadcaster).unwrap(),
                    None                  => ()
                }
            },
            Err(e) => panic!("{}", e)
        }
    }
}
