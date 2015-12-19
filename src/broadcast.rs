use std;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::{fmt, process, result, thread, time};
use std::os::unix::io::AsRawFd;
use std::sync::mpsc::{channel, Sender};

use pty;
use libc;
use libc_ext;
use nix::sys::signal;

use pty_spawn;
use winsize;
use message;

#[derive(Debug)]
pub enum Error {
    Message(message::Error),
    Pty(pty::Error),
    Io(io::Error),
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "Broadcast error"
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::Message(ref err) => Some(err),
            Error::Pty(ref err) => Some(err),
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

impl From<pty::Error> for Error {
    fn from(err: pty::Error) -> Error {
        Error::Pty(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

pub type Result<T> = result::Result<T, Error>;

static mut sigwinch_count: i32 = 0;
extern "C" fn handle_sigwinch(_: i32) {
    unsafe {
        sigwinch_count += 1;
    }
}

fn build_winsize_notification() -> Result<message::Notification> {
    let winsize = try!(winsize::from_fd(libc::STDIN_FILENO));
    let notification = message::Notification::Output(format!("\x1b[8;{};{}t",
                                                             winsize.ws_row,
                                                             winsize.ws_col));

    Ok(notification)
}

struct InputHandler {
    input: io::Stdin,
    child: pty::Child,
}

struct OutputHandler {
    output: io::Stdout,
    child: pty::Child,
    sender: Sender<Option<message::Notification>>,
}

struct ResizeHandler {
    child: pty::Child,
    sender: Sender<Option<message::Notification>>,
}

struct NotificationHandler {
    stream: TcpStream,
    sender: Sender<Option<message::Notification>>,
}

pub fn execute(host: String, port: i32, channel_name: String) -> Result<()> {
    let mut stream = try!(TcpStream::connect(&format!("{}:{}", host, port)[..]));
    let child = pty_spawn::pty_spawn();
    let (sender, receiver) = channel();

    let request = message::JoinRequest::Broadcast(channel_name.clone());

    try!(request.send(&mut stream));
    try!(message::JoinResponse::receive(&stream));

    InputHandler::spawn(io::stdin(), &child);
    OutputHandler::spawn(io::stdout(), &child, &sender);
    ResizeHandler::spawn(&child, &sender);
    NotificationHandler::spawn(&stream, &sender);

    let _ = sender.send(build_winsize_notification().ok());

    for message in receiver {
        match message {
            Some(notification) => try!(notification.send(&mut stream)),
            None => break,
        }
    }

    try!(child.wait());

    Ok(())
}

impl InputHandler {
    fn spawn(input: io::Stdin, child: &pty::Child) {
        let mut handler = InputHandler {
            input: input,
            child: child.clone(),
        };

        thread::spawn(move || {
            handler.process().unwrap_or_else(|e| {
                println!("{:?}", e);
                process::exit(1);
            });
        });
    }

    fn process(&mut self) -> Result<()> {
        let mut pty = self.child.pty().unwrap();
        let mut buf = [0; 128];

        loop {
            let nread = try!(self.input.read(&mut buf));

            try!(pty.write(&buf[..nread]));
        }
    }
}

impl OutputHandler {
    fn spawn(output: io::Stdout,
             child: &pty::Child,
             sender: &Sender<Option<message::Notification>>) {
        let mut handler = OutputHandler {
            output: output,
            child: child.clone(),
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
        let mut pty = self.child.pty().unwrap();
        let mut buf = [0; 1024 * 10];

        loop {
            let nread = pty.read(&mut buf).unwrap_or(0);

            if nread <= 0 {
                break;
            } else {
                try!(self.output.write(&buf[..nread]));
                let _ = self.output.flush();

                let string = String::from_utf8_lossy(&buf[..nread]).into_owned();
                let _ = self.sender.send(Some(message::Notification::Output(string)));
            }
        }

        let _ = self.sender.send(None);

        Ok(())
    }
}

impl ResizeHandler {
    fn spawn(child: &pty::Child, sender: &Sender<Option<message::Notification>>) {
        let handler = ResizeHandler {
            child: child.clone(),
            sender: sender.clone(),
        };

        Self::register_sigwinch_handler();

        thread::spawn(move || {
            handler.process().unwrap_or_else(|e| {
                println!("{:?}", e);
                process::exit(1);
            });
        });
    }

    fn register_sigwinch_handler() {
        let sig_action = signal::SigAction::new(handle_sigwinch,
                                                signal::signal::SA_RESTART,
                                                signal::SigSet::empty());

        unsafe {
            signal::sigaction(signal::SIGWINCH, &sig_action).unwrap();
        }
    }

    fn process(&self) -> Result<()> {
        let mut count = unsafe { sigwinch_count };

        loop {
            let last_count = unsafe { sigwinch_count };

            if last_count > count {
                let winsize = try!(winsize::from_fd(libc::STDIN_FILENO));

                self.handle_resize(&winsize);

                count = last_count;
            }

            thread::sleep(time::Duration::new(1, 0));
        }
    }

    fn handle_resize(&self, winsize: &libc_ext::Winsize) {
        let pty = self.child.pty().unwrap();

        let _ = self.sender.send(build_winsize_notification().ok());
        winsize::set(pty.as_raw_fd(), winsize);
    }
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
                    self.handle_closed(reason);

                    break;
                }
                message::Notification::WatcherJoined(_) => {
                    let _ = self.sender.send(build_winsize_notification().ok());
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn handle_closed(&self, reason: String) {
        let _ = self.sender.send(None);

        println!("Broadcast has stopped: {}", reason);
    }
}
