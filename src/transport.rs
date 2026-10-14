use crate::config::TransportKind;
use crate::kiss::{self, Decoder};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};
/// At most one background dial (including DNS). Failed attempts retry after 1 s.
/// Generation tags prevent a completed dial from restoring an obsolete configuration.
#[derive(Default)]
pub struct Connector {
    pending: Option<(u64, Receiver<io::Result<Transport>>)>,
    generation: u64,
    next_ms: u64,
}
impl Connector {
    pub fn reset(&mut self, next_ms: u64) {
        self.generation = self.generation.wrapping_add(1);
        self.next_ms = next_ms;
    }
    pub fn retry(&mut self, now: u64) {
        self.next_ms = now;
    }
    pub fn poll(
        &mut self,
        endpoint: &str,
        kind: TransportKind,
        baud: u32,
        now: u64,
    ) -> Option<io::Result<Transport>> {
        if let Some((generation, receiver)) = &self.pending {
            let result = match receiver.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => {
                    Err(io::Error::other("TCP connection worker stopped"))
                }
            };
            let current = *generation == self.generation;
            self.pending = None;
            if current {
                self.next_ms = now.saturating_add(1000);
                return Some(result);
            }
            // An obsolete socket is dropped here, never exposed to the caller.
        }
        if now >= self.next_ms {
            let (sender, receiver) = mpsc::sync_channel(1);
            let endpoint = endpoint.to_owned();
            match thread::Builder::new()
                .name("kiss-transport-connect".into())
                .spawn(move || {
                    let result = match kind {
                        TransportKind::Tcp => Transport::connect(&endpoint),
                        TransportKind::Serial => Transport::connect_serial(&endpoint, baud),
                    };
                    let _ = sender.send(result);
                }) {
                Ok(_) => self.pending = Some((self.generation, receiver)),
                Err(e) => {
                    self.next_ms = now.saturating_add(1000);
                    return Some(Err(e));
                }
            }
        }
        None
    }
}
pub struct Transport {
    io: TransportIo,
    decoder: Decoder,
    queue: VecDeque<Vec<u8>>,
    offset: usize,
    bytes: usize,
}
enum TransportIo {
    Tcp(TcpStream),
    #[cfg(windows)]
    Serial(SerialPort),
    #[cfg(unix)]
    Serial(std::fs::File),
}
impl Transport {
    pub fn connect(endpoint: &str) -> io::Result<Self> {
        let addresses = endpoint.to_socket_addrs()?;
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut error = io::Error::new(io::ErrorKind::InvalidInput, "No TNC address");
        for address in addresses {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match TcpStream::connect_timeout(&address, remaining) {
                Ok(stream) => {
                    stream.set_nonblocking(true)?;
                    stream.set_nodelay(true)?;
                    return Ok(Self {
                        io: TransportIo::Tcp(stream),
                        decoder: Decoder::default(),
                        queue: VecDeque::new(),
                        offset: 0,
                        bytes: 0,
                    });
                }
                Err(e) => error = e,
            }
        }
        Err(error)
    }

    pub fn connect_serial(device: &str, baud: u32) -> io::Result<Self> {
        #[cfg(windows)]
        {
            let serial = SerialPort::open(device, baud)?;
            Ok(Self {
                io: TransportIo::Serial(serial),
                decoder: Decoder::default(),
                queue: VecDeque::new(),
                offset: 0,
                bytes: 0,
            })
        }
        #[cfg(unix)]
        {
            let serial = crate::serial::open(device, baud)?;
            Ok(Self {
                io: TransportIo::Serial(serial),
                decoder: Decoder::default(),
                queue: VecDeque::new(),
                offset: 0,
                bytes: 0,
            })
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = (device, baud);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Serial TNC connections are currently supported on Windows only",
            ))
        }
    }

    pub fn available_serial_ports() -> io::Result<Vec<String>> {
        #[cfg(windows)]
        {
            SerialPort::available_ports()
        }
        #[cfg(not(windows))]
        {
            Ok(Vec::new())
        }
    }

    pub fn send(&mut self, port: u8, frame: &[u8]) -> io::Result<()> {
        let wire = kiss::encode(port, frame);
        if self.bytes + wire.len() > 65536 {
            return Err(io::Error::other("TNC TX queue exceeded 64 KiB"));
        }
        self.bytes += wire.len();
        self.queue.push_back(wire);
        Ok(())
    }
    pub fn poll(&mut self) -> io::Result<Vec<(u8, Vec<u8>)>> {
        while !self.queue.is_empty() {
            let remaining = self.queue.front().unwrap()[self.offset..].to_vec();
            let front_len = self.queue.front().unwrap().len();
            match self.write(&remaining) {
                Ok(0) if self.is_serial() => break,
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "TNC write returned zero",
                    ));
                }
                Ok(n) => {
                    self.offset += n;
                    self.bytes -= n;
                    if self.offset == front_len {
                        self.queue.pop_front();
                        self.offset = 0;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        let mut out = vec![];
        let mut buffer = [0; 4096];
        // Limit work per iteration so a busy monitor cannot starve session timers.
        for _ in 0..4 {
            match self.read(&mut buffer) {
                Ok(0) => {
                    if matches!(&self.io, TransportIo::Tcp(_)) {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "TNC closed TCP",
                        ));
                    }
                    break;
                }
                Ok(n) => {
                    for b in &buffer[..n] {
                        if let Some(frame) = self.decoder.feed(*b) {
                            out.push(frame);
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }

    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match &mut self.io {
            TransportIo::Tcp(stream) => stream.write(buffer),
            #[cfg(any(windows, unix))]
            TransportIo::Serial(port) => port.write(buffer),
        }
    }

    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match &mut self.io {
            TransportIo::Tcp(stream) => stream.read(buffer),
            #[cfg(any(windows, unix))]
            TransportIo::Serial(port) => port.read(buffer),
        }
    }

    fn is_serial(&self) -> bool {
        #[cfg(any(windows, unix))]
        return matches!(&self.io, TransportIo::Serial(_));
        #[cfg(not(any(windows, unix)))]
        false
    }
}

#[cfg(windows)]
struct SerialPort {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
unsafe impl Send for SerialPort {}

#[cfg(windows)]
impl SerialPort {
    fn open(device: &str, baud: u32) -> io::Result<Self> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Devices::Communication::{
                COMMTIMEOUTS, DCB, DCB_PARITY, DCB_STOP_BITS, GetCommState, NOPARITY, ONESTOPBIT,
                SetCommState, SetCommTimeouts,
            },
            Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE},
            Storage::FileSystem::{CreateFileW, OPEN_EXISTING},
        };

        if ![1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200].contains(&baud) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported baud rate",
            ));
        }
        let normalized = device.to_ascii_uppercase();
        if !normalized.starts_with("COM")
            || normalized[3..].is_empty()
            || !normalized[3..].bytes().all(|b| b.is_ascii_digit())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid COM port name",
            ));
        }
        let path = std::ffi::OsStr::new(&format!(r"\\.\{}", normalized))
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: path is NUL-terminated and valid for the duration of the call.
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let mut dcb = DCB {
            DCBlength: std::mem::size_of::<DCB>() as u32,
            ..DCB::default()
        };
        // SAFETY: handle is an open serial-device handle and dcb points to a writable DCB.
        let configured = unsafe { GetCommState(handle, &mut dcb) } != 0;
        if !configured {
            let error = io::Error::last_os_error();
            unsafe { CloseHandle(handle) };
            return Err(error);
        }
        dcb.BaudRate = baud;
        dcb.ByteSize = 8;
        dcb.Parity = NOPARITY as DCB_PARITY;
        dcb.StopBits = ONESTOPBIT as DCB_STOP_BITS;
        dcb._bitfield = 1; // Binary mode, 8N1, no software or hardware flow control.
        // SAFETY: handle remains open and dcb is initialized from GetCommState.
        if unsafe { SetCommState(handle, &dcb) } == 0 {
            let error = io::Error::last_os_error();
            unsafe { CloseHandle(handle) };
            return Err(error);
        }
        let timeouts = COMMTIMEOUTS {
            ReadIntervalTimeout: u32::MAX,
            ReadTotalTimeoutMultiplier: 0,
            ReadTotalTimeoutConstant: 0,
            WriteTotalTimeoutMultiplier: 0,
            WriteTotalTimeoutConstant: 100,
        };
        // SAFETY: handle is open and timeouts points to a valid structure.
        if unsafe { SetCommTimeouts(handle, &timeouts) } == 0 {
            let error = io::Error::last_os_error();
            unsafe { CloseHandle(handle) };
            return Err(error);
        }
        Ok(Self { handle })
    }

    fn available_ports() -> io::Result<Vec<String>> {
        use windows_sys::Win32::Storage::FileSystem::QueryDosDeviceW;
        let mut ports = Vec::new();
        for number in 1..=256 {
            let name = format!("COM{number}");
            let wide = name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            let mut target = [0u16; 512];
            // SAFETY: name is NUL-terminated and target is a writable output buffer.
            let result =
                unsafe { QueryDosDeviceW(wide.as_ptr(), target.as_mut_ptr(), target.len() as u32) };
            if result > 0 {
                ports.push(name);
            }
        }
        Ok(ports)
    }

    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        use windows_sys::Win32::Storage::FileSystem::WriteFile;
        let mut written = 0;
        // SAFETY: handle is open and buffer/written are valid for this synchronous call.
        if unsafe {
            WriteFile(
                self.handle,
                buffer.as_ptr(),
                buffer.len().min(u32::MAX as usize) as u32,
                &mut written,
                std::ptr::null_mut(),
            )
        } == 0
        {
            Err(io::Error::last_os_error())
        } else {
            Ok(written as usize)
        }
    }

    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        use windows_sys::Win32::Storage::FileSystem::ReadFile;
        let mut read = 0;
        // SAFETY: handle is open and buffer/read are valid for this synchronous call.
        if unsafe {
            ReadFile(
                self.handle,
                buffer.as_mut_ptr(),
                buffer.len().min(u32::MAX as usize) as u32,
                &mut read,
                std::ptr::null_mut(),
            )
        } == 0
        {
            Err(io::Error::last_os_error())
        } else {
            Ok(read as usize)
        }
    }
}

#[cfg(windows)]
impl Drop for SerialPort {
    fn drop(&mut self) {
        // SAFETY: this object exclusively owns the handle and drops it once.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_attempt_waits_one_second_and_obsolete_results_are_discarded() {
        let mut connector = Connector::default();
        let (tx, rx) = mpsc::sync_channel(1);
        connector.pending = Some((0, rx));
        tx.send(Err(io::Error::other("test refusal"))).unwrap();
        assert!(
            connector
                .poll("unused", TransportKind::Tcp, 9600, 50)
                .unwrap()
                .is_err()
        );
        assert_eq!(connector.next_ms, 1050);
        assert!(
            connector
                .poll("unused", TransportKind::Tcp, 9600, 1049)
                .is_none()
        );
        assert!(connector.pending.is_none());
        let (tx, rx) = mpsc::sync_channel(1);
        connector.pending = Some((connector.generation, rx));
        connector.reset(2000);
        tx.send(Err(io::Error::other("old endpoint"))).unwrap();
        assert!(
            connector
                .poll("unused", TransportKind::Tcp, 9600, 1000)
                .is_none()
        );
        assert!(connector.pending.is_none());
        assert_eq!(connector.next_ms, 2000);
    }
}
