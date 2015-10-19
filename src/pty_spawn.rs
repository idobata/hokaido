use libc;
use pty;
use std;
use termios::*;
use winsize;

pub fn pty_spawn() -> (pty::Child, Termios) {
    let termios = Termios::from_fd(libc::STDIN_FILENO).unwrap();
    let winsize = winsize::from_fd(libc::STDIN_FILENO).unwrap();
    let child   = pty::fork().unwrap();

    if child.pid() == 0 {
        tcsetattr(libc::STDIN_FILENO, TCSANOW, &termios).unwrap();
        winsize::set(libc::STDIN_FILENO, &winsize);

        exec_shell(std::env::var("SHELL").unwrap_or("bash".to_string()));

        panic!("Can't invoke new shell");
    } else {
        enter_raw_mode(libc::STDIN_FILENO);
    }

    (child, termios)
}

fn exec_shell(shell: String) {
    let cmd = std::ffi::CString::new(shell).unwrap();
    let mut args: Vec<*const libc::c_char> = Vec::with_capacity(1);

    args.push(cmd.as_ptr());
    args.push(std::ptr::null());

    unsafe { libc::execvp(cmd.as_ptr(), args.as_mut_ptr()) };
}

fn enter_raw_mode(fd: libc::c_int) {
    let mut new_termios = Termios::from_fd(fd).unwrap();

    new_termios.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
    new_termios.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
    new_termios.c_cflag &= !(CSIZE | PARENB);
    new_termios.c_cflag |= CS8;
    new_termios.c_oflag &= !(OPOST);
    new_termios.c_cc[VMIN]  = 1;
    new_termios.c_cc[VTIME] = 0;

    tcsetattr(libc::STDIN_FILENO, TCSANOW, &new_termios).unwrap();
}
