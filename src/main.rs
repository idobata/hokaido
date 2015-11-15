#![deny(missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unstable_features,
        unused_import_braces, unused_qualifications)]
#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

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

static USAGE: &'static str = "
Usage:
  hokaido <command> [--host=<host>] [--port=<port>] [--channel=<channel>]
  hokaido (-h | --help)
  hokaido (-v | --version)

Options:
  -h --help            Show this screen.
  -v --version         Show the version of hokaido.
  --host=<host>        Server name  [default: 0.0.0.0].
  --port=<port>        Server port  [default: 4423].
  --channel=<channel>  Channel Name [default: default].
";

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_command: Option<String>,
    flag_host: String,
    flag_port: i32,
    flag_channel: String,
    flag_version: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap_or_else(|e| e.exit());

    match args.arg_command {
        Some(command_name) => {
            match command_name.as_ref() {
                "broadcast" => broadcast::execute(args.flag_host, args.flag_port, args.flag_channel).unwrap_or_else(|e| panic!(e)),
                "server" => server::execute(args.flag_host, args.flag_port).unwrap_or_else(|e| panic!(e)),
                "watch" => watch::execute(args.flag_host, args.flag_port, args.flag_channel).unwrap_or_else(|e| panic!(e)),
                _ => unreachable!(),
            }
        }
        _ => {
            if args.flag_version {
                println!("{}", env!("CARGO_PKG_VERSION"));
            } else {
                unreachable!();
            }
        }
    }
}
