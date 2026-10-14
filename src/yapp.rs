//! Classic YAPP 1.1 packet framing carried inside an AX.25 connected-mode
//! information stream. This module deliberately does not implement YAPP-C.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    SendInit,
    Ready,
    ReceiveFile,
    AckEof,
    AckEot,
    CanAck,
    Header { filename: String, size: u64 },
    Data(Vec<u8>),
    Eof,
    Eot,
    NotReady(Vec<u8>),
    Cancel(Vec<u8>),
}

#[derive(Default, Debug)]
pub struct Decoder {
    buffer: Vec<u8>,
}

impl Decoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Packet>, String> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > 16 * 1024 {
            return Err("YAPP control packet exceeds 16 KiB".into());
        }
        let mut packets = Vec::new();
        while let Some((&kind, rest)) = self.buffer.split_first() {
            let length = match kind {
                0x03..=0x06 => 2,
                0x01 | 0x02 | 0x15 | 0x18 => {
                    if rest.is_empty() {
                        break;
                    }
                    let body = if kind == 0x02 && rest[0] == 0 {
                        256
                    } else {
                        rest[0] as usize
                    };
                    2 + body
                }
                _ => return Err(format!("Unexpected YAPP packet type 0x{kind:02x}")),
            };
            if self.buffer.len() < length {
                break;
            }
            let frame = self.buffer.drain(..length).collect::<Vec<_>>();
            let packet = match frame[0] {
                0x05 if frame[1] == 0x01 => Packet::SendInit,
                0x06 if frame[1] == 0x01 => Packet::Ready,
                0x06 if frame[1] == 0x02 => Packet::ReceiveFile,
                0x06 if frame[1] == 0x03 => Packet::AckEof,
                0x06 if frame[1] == 0x04 => Packet::AckEot,
                0x06 if frame[1] == 0x05 => Packet::CanAck,
                0x05 | 0x06 => return Err("Invalid fixed-size YAPP packet".into()),
                0x01 => {
                    let fields = &frame[2..];
                    let name_end = fields
                        .iter()
                        .position(|byte| *byte == 0)
                        .ok_or("YAPP header has no filename terminator")?;
                    let remainder = &fields[name_end + 1..];
                    let size_end = remainder
                        .iter()
                        .position(|byte| *byte == 0)
                        .ok_or("YAPP header has no file-size terminator")?;
                    if remainder[size_end + 1..]
                        .iter()
                        .any(|byte| !byte.is_ascii() || (*byte < 0x20 && *byte != 0))
                    {
                        return Err("YAPP header extension contains invalid bytes".into());
                    }
                    let filename = std::str::from_utf8(&fields[..name_end])
                        .map_err(|_| "YAPP filename is not valid UTF-8")?
                        .to_owned();
                    let size = std::str::from_utf8(&remainder[..size_end])
                        .map_err(|_| "YAPP file size is not ASCII")?
                        .parse::<u64>()
                        .map_err(|_| "Invalid YAPP file size")?;
                    Packet::Header { filename, size }
                }
                0x02 => {
                    let count = if frame[1] == 0 {
                        256
                    } else {
                        frame[1] as usize
                    };
                    if frame.len() != count + 2 {
                        return Err("Invalid YAPP data length".into());
                    }
                    Packet::Data(frame[2..].to_vec())
                }
                0x03 if frame[1] == 0x01 => Packet::Eof,
                0x04 if frame[1] == 0x01 => Packet::Eot,
                0x03 | 0x04 => return Err("Invalid YAPP end packet".into()),
                0x15 => Packet::NotReady(frame[2..].to_vec()),
                0x18 => Packet::Cancel(frame[2..].to_vec()),
                _ => unreachable!(),
            };
            packets.push(packet);
        }
        Ok(packets)
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

pub fn control(kind: u8) -> Vec<u8> {
    vec![0x06, kind]
}

pub fn not_ready(reason: &str) -> Vec<u8> {
    let text = reason.as_bytes();
    let text = &text[..text.len().min(u8::MAX as usize)];
    let mut packet = vec![0x15, text.len() as u8];
    packet.extend_from_slice(text);
    packet
}

pub fn send_init() -> [u8; 2] {
    [0x05, 0x01]
}

pub fn header(filename: &str, size: u64) -> Result<Vec<u8>, String> {
    if filename.is_empty() || !filename.is_ascii() || filename.as_bytes().contains(&0) {
        return Err("YAPP filename must be nonempty ASCII without NUL".into());
    }
    let mut fields = filename.as_bytes().to_vec();
    fields.push(0);
    fields.extend_from_slice(size.to_string().as_bytes());
    fields.push(0);
    if fields.len() > u8::MAX as usize {
        return Err("YAPP header is too long".into());
    }
    let mut packet = vec![0x01, fields.len() as u8];
    packet.extend(fields);
    Ok(packet)
}

pub fn data(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.is_empty() || bytes.len() > 256 {
        return Err("YAPP data packet must contain 1-256 bytes".into());
    }
    let mut packet = vec![
        0x02,
        if bytes.len() == 256 {
            0
        } else {
            bytes.len() as u8
        },
    ];
    packet.extend_from_slice(bytes);
    Ok(packet)
}

pub fn end_of_file() -> [u8; 2] {
    [0x03, 0x01]
}

pub fn end_of_transfer() -> [u8; 2] {
    [0x04, 0x01]
}

pub fn cancel(reason: &str) -> Vec<u8> {
    let text = reason.as_bytes();
    let text = &text[..text.len().min(u8::MAX as usize)];
    let mut packet = vec![0x18, text.len() as u8];
    packet.extend_from_slice(text);
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_packets_split_across_ax25_information_frames() {
        let header = header("packet.bin", 513).unwrap();
        let mut decoder = Decoder::default();
        assert!(decoder.push(&header[..4]).unwrap().is_empty());
        assert_eq!(
            decoder.push(&header[4..]).unwrap(),
            vec![Packet::Header {
                filename: "packet.bin".into(),
                size: 513,
            }]
        );
        assert_eq!(decoder.push(&[0x05]).unwrap(), Vec::<Packet>::new());
        assert_eq!(
            decoder.push(&[0x01, 0x06, 0x04]).unwrap(),
            vec![Packet::SendInit, Packet::AckEot]
        );
    }

    #[test]
    fn data_packet_zero_length_encodes_256_octets() {
        let packet = data(&vec![0x5a; 256]).unwrap();
        let mut decoder = Decoder::default();
        assert_eq!(
            decoder.push(&packet).unwrap(),
            vec![Packet::Data(vec![0x5a; 256])]
        );
    }

    #[test]
    fn accepts_optional_header_fields_without_negotiating_yapp_c() {
        let mut decoder = Decoder::default();
        assert_eq!(
            decoder
                .push(&[0x01, 7, b'a', 0, b'1', 0, b'X', b'Y', 0])
                .unwrap(),
            vec![Packet::Header {
                filename: "a".into(),
                size: 1
            }]
        );
    }
}
