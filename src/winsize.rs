use libc;
use libc_ext::{TIOCGWINSZ, TIOCSWINSZ, Winsize};
use libc::funcs::bsd44::ioctl;
use std::io;

pub fn from_fd(fd: libc::c_int) -> io::Result<Winsize> {
    let winsize = Winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };

    unsafe { ioctl(fd, TIOCGWINSZ, &winsize) };

    Ok(winsize)
}

pub fn set(fd: libc::c_int, winsize: &Winsize) {
    unsafe { ioctl(fd, TIOCSWINSZ, winsize); };
}
