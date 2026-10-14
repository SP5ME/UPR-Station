//! Streaming KISS codec. AX.25 FCS belongs to the TNC, not this layer.
const LIMIT: usize = 4096;
#[derive(Default)]
pub struct Decoder {
    bytes: Vec<u8>,
    escaped: bool,
    discard: bool,
    started: bool,
}
impl Decoder {
    pub fn feed(&mut self, byte: u8) -> Option<(u8, Vec<u8>)> {
        if byte == 0xc0 {
            let result = if self.started && !self.discard && !self.escaped && !self.bytes.is_empty()
            {
                Some((self.bytes[0], self.bytes[1..].to_vec()))
            } else {
                None
            };
            self.bytes.clear();
            self.escaped = false;
            self.discard = false;
            self.started = true;
            return result;
        }
        if !self.started || self.discard {
            return None;
        }
        let decoded = if self.escaped {
            self.escaped = false;
            match byte {
                0xdc => 0xc0,
                0xdd => 0xdb,
                // The KISS description specifies that an invalid escaped byte
                // is ignored and frame assembly continues.
                _ => return None,
            }
        } else if byte == 0xdb {
            self.escaped = true;
            return None;
        } else {
            byte
        };
        if self.bytes.len() == LIMIT {
            self.bytes.clear();
            self.discard = true;
        } else {
            self.bytes.push(decoded);
        }
        None
    }
}
pub fn encode(port: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0xc0];
    for b in std::iter::once(port << 4).chain(payload.iter().copied()) {
        match b {
            0xc0 => out.extend([0xdb, 0xdc]),
            0xdb => out.extend([0xdb, 0xdd]),
            _ => out.push(b),
        }
    }
    out.push(0xc0);
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes_and_stream_boundaries() {
        let mut d = Decoder::default();
        let wire = encode(12, &[0xc0, 0xdb, 0, 255]);
        let frames: Vec<_> = wire.into_iter().filter_map(|b| d.feed(b)).collect();
        assert_eq!(frames, vec![(0xc0, vec![0xc0, 0xdb, 0, 255])]);
    }
    #[test]
    fn oversized_frame_recovers() {
        let mut d = Decoder::default();
        d.feed(0xc0);
        for _ in 0..5000 {
            assert!(d.feed(1).is_none());
        }
        assert!(d.feed(0xc0).is_none());
        let got: Vec<_> = encode(0, &[42])
            .into_iter()
            .filter_map(|b| d.feed(b))
            .collect();
        assert_eq!(got, vec![(0, vec![42])]);
    }
}
