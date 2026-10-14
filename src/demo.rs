//! Loopback-only demonstration peer; never connects to a radio.
use crate::{
    ax25::Frame,
    kiss::{self, Decoder},
    link::{Engine, Event, State},
};
use std::{
    io::{self, Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};
fn output(
    events: Vec<Event>,
    engine: &mut Engine,
    stream: &mut std::net::TcpStream,
    now: u64,
) -> io::Result<()> {
    let mut work = std::collections::VecDeque::from(events);
    while let Some(event) = work.pop_front() {
        match event {
            Event::Established(_) => {}
            Event::Transmit(f) | Event::TransmitOn(_, f) => {
                stream.write_all(&kiss::encode(0, &f.encode()))?
            }
            Event::Data(id, data) => {
                let mut reply = b"Echo: ".to_vec();
                reply.extend(data);
                work.extend(engine.send(id, &reply, now).map_err(io::Error::other)?);
            }
            Event::Notice(id, text) => println!("[{}] {text}", id + 1),
            Event::Status(id, status) => {
                println!("[{}] {status}", id + 1);
                if engine.sessions[id]
                    .as_ref()
                    .is_some_and(|s| s.state == State::Connected)
                {
                    work.extend(
                        engine
                            .send(
                                id,
                                b"Welcome to local DEMO echo node. No radio attached.\r",
                                now,
                            )
                            .map_err(io::Error::other)?,
                    );
                }
            }
        }
    }
    Ok(())
}
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
pub fn run(listener: TcpListener, stop: Arc<AtomicBool>) -> io::Result<()> {
    listener.set_nonblocking(true)?;
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false)?;
                let _ = serve(stream, &stop);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
fn serve(mut stream: std::net::TcpStream, stop: &AtomicBool) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(100)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    let mut engine = Engine::new("DEMO".parse().unwrap());
    let mut decoder = Decoder::default();
    let start = Instant::now();
    let mut buf = [0; 4096];
    while !stop.load(Ordering::Relaxed) {
        let now = start.elapsed().as_millis() as u64;
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                for b in &buf[..n] {
                    if let Some((0, data)) = decoder.feed(*b)
                        && let Ok(source) = Frame::peek_source(&data)
                        && let Ok(frame) =
                            Frame::decode_with_mode(&data, engine.extended_for(&source))
                    {
                        let events = engine.receive(frame, now);
                        output(events, &mut engine, &mut stream, now)?;
                    }
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) => {}
            Err(e) => return Err(e),
        }
        let events = engine.tick(now);
        output(events, &mut engine, &mut stream, now)?;
    }
    Ok(())
}
