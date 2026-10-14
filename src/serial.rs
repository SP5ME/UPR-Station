//! Linux/Unix serial KISS transport: nonblocking 8N1, no flow control.
use std::{
    fs::{File, OpenOptions},
    io,
    os::{
        fd::AsRawFd,
        unix::fs::{FileTypeExt, OpenOptionsExt},
    },
};
pub fn open(device: &str, baud: u32) -> io::Result<File> {
    let speed = match baud {
        1200 => libc::B1200,
        2400 => libc::B2400,
        4800 => libc::B4800,
        9600 => libc::B9600,
        19200 => libc::B19200,
        38400 => libc::B38400,
        57600 => libc::B57600,
        115200 => libc::B115200,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported baud rate",
            ));
        }
    };
    if !device.starts_with("/dev/") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use a /dev/ serial device on Linux",
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(device)?;
    if !file.metadata()?.file_type().is_char_device() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Not a character device",
        ));
    }
    let fd = file.as_raw_fd();
    // termios is owned by this call; all FFI results are checked before use.
    unsafe {
        let mut attributes = std::mem::zeroed::<libc::termios>();
        if libc::tcgetattr(fd, &mut attributes) < 0 {
            return Err(io::Error::last_os_error());
        }
        libc::cfmakeraw(&mut attributes);
        attributes.c_cflag &= !(libc::PARENB | libc::CSTOPB | libc::CSIZE | libc::CRTSCTS);
        attributes.c_cflag |= libc::CS8 | libc::CLOCAL | libc::CREAD;
        attributes.c_cc[libc::VMIN] = 0;
        attributes.c_cc[libc::VTIME] = 0;
        if libc::cfsetispeed(&mut attributes, speed) < 0
            || libc::cfsetospeed(&mut attributes, speed) < 0
            || libc::tcsetattr(fd, libc::TCSANOW, &attributes) < 0
        {
            return Err(io::Error::last_os_error());
        }
        if libc::ioctl(fd, libc::TIOCEXCL) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(file)
}
