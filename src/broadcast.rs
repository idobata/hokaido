use std::io::{Read, Write};
use std::net::TcpStream;
use std::{io, thread};
use std::os::unix::io::{AsRawFd};
use std::sync::mpsc::{channel, Sender};

use pty;
use libc;
use libc_ext;
use nix::sys::signal;
use termios::*;

use pty_spawn;
use winsize;
use message;

pub fn execute(host: String, port: i32, channel_name: String) {
    let mut stream         = TcpStream::connect(&format!("{}:{}", host, port)[..]).unwrap();
    let (child, termios)   = pty_spawn::pty_spawn();
    let (sender, receiver) = channel();

    let request = message::JoinRequest::Broadcast(channel_name.clone());
    request.send(&mut stream).unwrap();
    message::JoinResponse::receive(&stream).unwrap();

    InputHandler::spawn(io::stdin(), &child);
    OutputHandler::spawn(io::stdout(), &child, &sender);
    ResizeHandler::spawn(&child, &sender);
    NotificationHandler::spawn(&stream, &sender);

    for message in receiver {
        match message {
            Some(notification) => notification.send(&mut stream).unwrap(),
            None => break
        };
    }

    child.wait().unwrap();
    tcsetattr(libc::STDIN_FILENO, TCSANOW, &termios).unwrap();
}

static mut sigwinch_count: i32 = 0;
extern fn handle_sigwinch(_: i32) {
    unsafe { sigwinch_count += 1; };
}

struct InputHandler {
    input: io::Stdin,
    child: pty::Child,
}

struct OutputHandler {
    output: io::Stdout,
    child:  pty::Child,
    sender: Sender<Option<message::Notification>>,
}

struct ResizeHandler {
    child:  pty::Child,
    sender: Sender<Option<message::Notification>>,
}

struct NotificationHandler {
    stream: TcpStream,
    sender: Sender<Option<message::Notification>>,
}

impl InputHandler {
    fn spawn(input: io::Stdin, child: &pty::Child) {
        let mut handler = InputHandler { input: input, child: child.clone() };

        thread::spawn(move || {
            handler.process();
        });
    }

    fn process(&mut self) {
        let mut pty = self.child.pty().unwrap();

        loop {
            let mut buf = vec![0; 128];
            let nread   = self.input.read(&mut buf[..]).unwrap();

            buf.truncate(nread as usize);
            pty.write(&buf).unwrap();
        }
    }
}

impl OutputHandler {
    fn spawn(output: io::Stdout, child: &pty::Child, sender: &Sender<Option<message::Notification>>) {
        let mut handler = OutputHandler { output: output, child: child.clone(), sender: sender.clone() };

        thread::spawn(move || {
            handler.process();
        });
    }

    fn process(&mut self) {
        let mut pty = self.child.pty().unwrap();

        loop {
            let mut buf = vec![0; 128];

            if pty.read(&mut buf).unwrap() <= 0 {
                break;
            } else {
                let string = String::from_utf8_lossy(&buf[..]).into_owned();

                self.handle_output(&string);
            }
        }

        self.handle_terminate();
    }

    fn handle_output(&mut self, string: &String) {
        let notification = message::Notification::Output(string.clone());

        print!("{}", string);
        self.output.flush().unwrap();

        self.sender.send(Some(notification)).unwrap();
    }

    fn handle_terminate(&self) {
        self.sender.send(None).unwrap();
    }
}

impl ResizeHandler {
    fn spawn(child: &pty::Child, sender: &Sender<Option<message::Notification>>) {
        let handler = ResizeHandler { child: child.clone(), sender: sender.clone() };

        Self::register_sigwinch_handler();

        thread::spawn(move || {
            handler.process();
        });
    }

    fn register_sigwinch_handler() {
        let sig_action = signal::SigAction::new(
            handle_sigwinch,
            signal::signal::SA_RESTART,
            signal::SigSet::empty()
        );

        unsafe { signal::sigaction(signal::SIGWINCH, &sig_action).unwrap(); };
    }

    fn process(&self) {
        let mut count = unsafe { sigwinch_count };

        loop {
            let last_count = unsafe { sigwinch_count };

            if last_count > count {
                let winsize = winsize::from_fd(libc::STDIN_FILENO).unwrap();

                self.handle_resize(&winsize);

                count = last_count;
            }

            thread::sleep_ms(1000);
        }
    }

    fn handle_resize(&self, winsize: &libc_ext::Winsize) {
        let pty          = self.child.pty().unwrap();
        let notification = message::Notification::Output(format!("\x1b[8;{};{}t", winsize.ws_row, winsize.ws_col));

        self.sender.send(Some(notification)).unwrap();
        winsize::set(pty.as_raw_fd(), winsize);
    }
}

impl NotificationHandler {
    fn spawn(stream: &TcpStream, sender: &Sender<Option<message::Notification>>) {
        let mut handler = NotificationHandler { stream: stream.try_clone().unwrap(), sender: sender.clone() };

        thread::spawn(move || {
            handler.process();
        });
    }

    fn process(&mut self) {
        loop {
            let notification = message::Notification::receive(&self.stream).unwrap();

            match notification {
                message::Notification::Closed(reason) => {
                    self.handle_closed(&reason);

                    break;
                },
                _ => continue
            }
        }
    }

    fn handle_closed(&self, reason: &String) {
        self.sender.send(None).unwrap();

        println!("Broadcast has stopped: {}", reason);
    }
}
