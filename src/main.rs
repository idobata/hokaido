extern crate docopt;
extern crate libc;
extern crate nix;
extern crate pty;
extern crate rmp_serialize as msgpack;
extern crate rustc_serialize;
extern crate termios;

mod libc_ext;
mod pty_spawn;
mod winsize;
mod message;
mod broadcast;
mod server;
mod watch;

use docopt::Docopt;

static USAGE: &'static str ="
Usage:
  hokaido <command>
  hokaido <command> [--host=<host>] [--port=<port>] [--channel=<channel>]
  hokaido (-h | --help)
  hokaido (-v | --version)

Options:
  -h --help            Show this screen.
  -v --version         Show the version of hokaido.
  --host=<host>        Server name     [default: localhost].
  --port=<port>        Port number     [default: 4423].
  --channel=<channel>  channelnel name [default: default].
";

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_command:  Option<String>,
    flag_host:    String,
    flag_port:    i32,
    flag_channel: String,
    flag_version: bool
}

fn main() {
    let args: Args = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap_or_else(|e| e.exit() );

    match args.arg_command {
        Some(command_name) => {
            match command_name.as_ref() {
                "broadcast" => broadcast::execute(args.flag_host, args.flag_port, args.flag_channel),
                "server"    => server::execute(args.flag_host, args.flag_port),
                "watch"     => watch::execute(args.flag_host, args.flag_port, args.flag_channel),
                _           => unreachable!()
            }
        },
        _ => {
            if args.flag_version {
                println!("{}", env!("CARGO_PKG_VERSION"));
            } else {
                unreachable!();
            }
        }
    }
}
