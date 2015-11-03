use libc;

#[cfg(target_os="macos")]
pub const TIOCGWINSZ: libc::c_ulonglong = 0x5413;
#[cfg(target_os="macos")]
pub const TIOCSWINSZ: libc::c_ulonglong = 0x5414;

#[cfg(target_os="linux")]
pub const TIOCGWINSZ: libc::c_int = 0x5413;
#[cfg(target_os="linux")]
pub const TIOCSWINSZ: libc::c_int = 0x5414;

#[repr(C)]
#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug)]
pub struct Winsize {
    pub ws_row: libc::c_ushort, // rows, in characters
    pub ws_col: libc::c_ushort, // columns, in characters
    pub ws_xpixel: libc::c_ushort, // horizontal size, pixels
    pub ws_ypixel: libc::c_ushort, // vertical size, pixels
}
