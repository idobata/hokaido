use libc;

pub const TIOCGWINSZ: libc::c_int = 0x5413;
pub const TIOCSWINSZ: libc::c_int = 0x5414;

#[repr(C)]
#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug)]
pub struct Winsize {
    pub ws_row: libc::c_ushort,    /* rows, in characters */
    pub ws_col: libc::c_ushort,    /* columns, in characters */
    pub ws_xpixel: libc::c_ushort, /* horizontal size, pixels */
    pub ws_ypixel: libc::c_ushort  /* vertical size, pixels */
}
