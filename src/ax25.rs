use std::{fmt, str::FromStr};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    name: String,
    ssid: u8,
}
impl FromStr for Call {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let s = s.to_ascii_uppercase();
        let mut parts = s.split('-');
        let name = parts.next().unwrap_or_default();
        let ssid = match parts.next() {
            Some(v) => v.parse::<u8>().map_err(|_| "Invalid SSID")?,
            None => 0,
        };
        if name.is_empty()
            || name.len() > 6
            || !name
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            || ssid > 15
            || parts.next().is_some()
        {
            return Err("Call must be 1-6 letters/digits with optional SSID 0-15".into());
        }
        Ok(Self {
            name: name.into(),
            ssid,
        })
    }
}
impl fmt::Display for Call {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if self.ssid != 0 {
            write!(f, "-{}", self.ssid)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub destination: Call,
    pub source: Call,
    pub path: Vec<(Call, bool)>,
    pub command: bool,
    pub response: bool,
    /// Control field bits in wire order (first octet in bits 0..7).
    pub control: u16,
    /// Whether this I/S frame uses the AX.25 extended two-octet control field.
    /// U frames remain one octet even while a link is in modulo-128 mode.
    pub extended: bool,
    pub pid: Option<u8>,
    pub info: Vec<u8>,
}
impl Frame {
    /// Read the destination address without interpreting a mode-dependent control field.
    pub fn peek_destination(bytes: &[u8]) -> Result<Call, &'static str> {
        let raw = bytes.get(..7).ok_or("Invalid address length")?;
        if raw[..6].iter().any(|b| b & 1 != 0) {
            return Err("Invalid address encoding");
        }
        let name: String = raw[..6].iter().map(|b| (b >> 1) as char).collect();
        format!("{}-{}", name.trim_end(), (raw[6] >> 1) & 15)
            .parse()
            .map_err(|_| "Invalid call")
    }

    /// Read the source address without interpreting a mode-dependent control field.
    pub fn peek_source(bytes: &[u8]) -> Result<Call, &'static str> {
        let mut pos = 0;
        for index in 0..2 {
            if bytes.len() < pos + 7 {
                return Err("Invalid address length");
            }
            let raw = &bytes[pos..pos + 7];
            if raw[..6].iter().any(|b| b & 1 != 0) {
                return Err("Invalid address encoding");
            }
            let name: String = raw[..6].iter().map(|b| (b >> 1) as char).collect();
            let call = format!("{}-{}", name.trim_end(), (raw[6] >> 1) & 15)
                .parse()
                .map_err(|_| "Invalid call")?;
            if index == 1 {
                return Ok(call);
            }
            pos += 7;
        }
        Err("Missing source address")
    }

    pub fn pf(&self) -> bool {
        if self.extended && (self.control & 1 == 0 || self.control & 3 == 1) {
            self.control & 0x0100 != 0
        } else {
            self.control & 0x0010 != 0
        }
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let addresses = std::iter::once((&self.destination, self.command))
            .chain(std::iter::once((&self.source, self.response)))
            .chain(self.path.iter().map(|(c, h)| (c, *h)))
            .collect::<Vec<_>>();
        for (i, (call, flag)) in addresses.iter().enumerate() {
            for j in 0..6 {
                out.push(call.name.as_bytes().get(j).copied().unwrap_or(b' ') << 1);
            }
            out.push(
                0x60 | call.ssid << 1 | u8::from(*flag) << 7 | u8::from(i + 1 == addresses.len()),
            );
        }
        out.push(self.control as u8);
        if self.extended && (self.control & 1 == 0 || self.control & 3 == 1) {
            out.push((self.control >> 8) as u8);
        }
        if let Some(pid) = self.pid {
            out.push(pid);
        }
        out.extend(&self.info);
        out
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        Self::decode_with_mode(bytes, false)
    }

    /// Decode using the sequence mode already established for the link. U frames
    /// are always decoded with a one-octet control field.
    pub fn decode_with_mode(bytes: &[u8], extended: bool) -> Result<Self, &'static str> {
        let mut addresses = Vec::new();
        let mut pos = 0;
        loop {
            if addresses.len() == 10 || bytes.len() < pos + 7 {
                return Err("Invalid address length");
            }
            let raw = &bytes[pos..pos + 7];
            if raw[..6].iter().any(|b| b & 1 != 0) {
                return Err("Invalid address encoding");
            }
            let name: String = raw[..6].iter().map(|b| (b >> 1) as char).collect();
            let call = format!("{}-{}", name.trim_end(), (raw[6] >> 1) & 15)
                .parse()
                .map_err(|_| "Invalid call")?;
            addresses.push((call, raw[6] & 0x80 != 0));
            pos += 7;
            if raw[6] & 1 != 0 {
                break;
            }
        }
        if addresses.len() < 2 || pos >= bytes.len() {
            return Err("Missing address/control");
        }
        let first_control = bytes[pos];
        pos += 1;
        let has_extended_control = extended && (first_control & 1 == 0 || first_control & 3 == 1);
        let control = if has_extended_control {
            let second = *bytes.get(pos).ok_or("Missing extended control octet")?;
            pos += 1;
            u16::from(first_control) | (u16::from(second) << 8)
        } else {
            u16::from(first_control)
        };
        let pid = if first_control & 1 == 0 || first_control & 0xef == 3 {
            if pos >= bytes.len() {
                return Err("Missing PID");
            }
            let p = bytes[pos];
            pos += 1;
            Some(p)
        } else {
            None
        };
        if bytes.len() - pos > 256 {
            return Err("Information field too long");
        }
        if first_control & 3 == 1 && pos != bytes.len() {
            return Err("Supervisory frame with information");
        }
        let (destination, command) = addresses.remove(0);
        let (source, response) = addresses.remove(0);
        Ok(Self {
            destination,
            source,
            path: addresses,
            command,
            response,
            control,
            extended: has_extended_control,
            pid,
            info: bytes[pos..].to_vec(),
        })
    }
    pub fn summary(&self) -> String {
        let c = self.control;
        let kind = if c & 1 == 0 {
            if self.extended {
                format!("I N(S)={} N(R)={}", (c >> 1) & 0x7f, (c >> 9) & 0x7f)
            } else {
                format!("I N(S)={} N(R)={}", (c >> 1) & 7, c >> 5)
            }
        } else if c & 3 == 1 {
            let (function, nr) = if self.extended {
                (c & 0x0f, (c >> 9) & 0x7f)
            } else {
                (c & 15, c >> 5)
            };
            format!(
                "{} N(R)={}",
                match function {
                    1 => "RR",
                    5 => "RNR",
                    9 => "REJ",
                    _ => "SREJ",
                },
                nr
            )
        } else {
            match c & 0xef {
                0x2f => "SABM",
                // SABME selects AX.25 extended sequence numbering (modulo 128).
                // Label the mode explicitly in the monitor so an operator can
                // distinguish it from the supported SABM/modulo-8 handshake.
                0x6f => "SABME (modulo 128)",
                0x63 => "UA",
                0x43 => "DISC",
                0x0f => "DM",
                3 => "UI",
                0x87 => "FRMR",
                0xaf => "XID",
                _ => "U?",
            }
            .into()
        };
        let via = self
            .path
            .iter()
            .map(|(c, h)| format!(" {}{}", c, if *h { "*" } else { "" }))
            .collect::<String>();
        format!(
            "{} > {}{} [{} {} P/F={}] {} bytes",
            self.source,
            self.destination,
            via,
            kind,
            if self.command && !self.response {
                "C"
            } else {
                "R"
            },
            u8::from(self.pf()),
            self.info.len()
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_address_vector() {
        let f = Frame {
            destination: "CQ".parse().unwrap(),
            source: "N0CALL-1".parse().unwrap(),
            path: vec![],
            command: true,
            response: false,
            control: 3,
            extended: false,
            pid: Some(240),
            info: b"hello".to_vec(),
        };
        let b = f.encode();
        assert_eq!(&b[..7], &[0x86, 0xa2, 0x40, 0x40, 0x40, 0x40, 0xe0]);
        assert_eq!(b[13], 0x63);
        let d = Frame::decode(&b).unwrap();
        assert_eq!(d.source, f.source);
        assert_eq!(d.info, b"hello");
        for n in 0..15 {
            assert!(Frame::decode(&b[..n]).is_err());
        }
    }

    #[test]
    fn monitor_identifies_extended_mode_in_sabme() {
        let frame = Frame {
            destination: "N0CALL".parse().unwrap(),
            source: "N0CALL-2".parse().unwrap(),
            path: vec![],
            command: true,
            response: false,
            control: 0x6f,
            extended: false,
            pid: None,
            info: vec![],
        };

        assert!(frame.summary().contains("SABME (modulo 128)"));
    }

    #[test]
    fn extended_control_field_round_trips_i_and_s_frames() {
        for control in [(100u16 << 1) | (77u16 << 9), 1 | (73u16 << 9) | 0x0100] {
            let frame = Frame {
            destination: "N0CALL".parse().unwrap(),
            source: "N0CALL-2".parse().unwrap(),
                path: vec![],
                command: true,
                response: false,
                control,
                extended: true,
                pid: if control & 1 == 0 { Some(0xf0) } else { None },
                info: if control & 1 == 0 {
                    b"extended payload".to_vec()
                } else {
                    vec![]
                },
            };
            let decoded = Frame::decode_with_mode(&frame.encode(), true).unwrap();
            assert_eq!(decoded.control, frame.control);
            assert_eq!(decoded.pid, frame.pid);
            assert_eq!(decoded.info, frame.info);
            assert_eq!(decoded.pf(), frame.pf());
        }
    }

    #[test]
    fn u_frames_preserve_pf_and_command_response_bits() {
        for (control, command, expected_pf) in [
            (0x3f, true, true),  // SABM, P=1, command
            (0x7f, true, true),  // SABME, P=1, command
            (0x73, false, true), // UA, F=1, response
            (0x1f, false, true), // DM, F=1, response
            (0xbf, true, true),  // XID, P=1, command
        ] {
            let frame = Frame {
                destination: "LOCAL".parse().unwrap(),
                source: "REMOTE".parse().unwrap(),
                path: vec![],
                command,
                response: !command,
                control,
                extended: false,
                pid: None,
                info: if control & 0xef == 0xaf {
                    vec![0x82, 0x80, 0, 0]
                } else {
                    vec![]
                },
            };
            let encoded = frame.encode();
            assert_eq!(encoded[6] & 0x80 != 0, command);
            assert_eq!(encoded[13] & 0x80 != 0, !command);
            let decoded = Frame::decode(&encoded).unwrap();
            assert_eq!(decoded.command, command);
            assert_eq!(decoded.response, !command);
            assert_eq!(decoded.pf(), expected_pf);
            assert_eq!(decoded.control, control);
        }
    }

    #[test]
    fn control_lengths_and_sequence_numbers_wrap_in_both_modes() {
        let modulo8 = Frame {
            destination: "LOCAL".parse().unwrap(),
            source: "REMOTE".parse().unwrap(),
            path: vec![],
            command: true,
            response: false,
            control: (7 << 1) | (7 << 5),
            extended: false,
            pid: Some(0xf0),
            info: b"8".to_vec(),
        };
        let wire8 = modulo8.encode();
        assert_eq!(wire8.len(), 17);
        let decoded8 = Frame::decode_with_mode(&wire8, false).unwrap();
        assert_eq!((decoded8.control >> 1) & 7, 7);
        assert_eq!((decoded8.control >> 5) & 7, 7);

        let modulo128 = Frame {
            control: (127 << 1) | (127 << 9),
            extended: true,
            pid: Some(0xf0),
            info: b"128".to_vec(),
            ..modulo8.clone()
        };
        let wire128 = modulo128.encode();
        assert_eq!(wire128.len(), 20);
        let decoded128 = Frame::decode_with_mode(&wire128, true).unwrap();
        assert_eq!((decoded128.control >> 1) & 0x7f, 127);
        assert_eq!((decoded128.control >> 9) & 0x7f, 127);
    }

    #[test]
    fn malformed_extended_control_field_is_rejected() {
        let frame = Frame {
            destination: "LOCAL".parse().unwrap(),
            source: "REMOTE".parse().unwrap(),
            path: vec![],
            command: true,
            response: false,
            control: 0,
            extended: true,
            pid: Some(0xf0),
            info: vec![],
        };
        let mut wire = frame.encode();
        wire.truncate(wire.len() - 1);
        assert!(Frame::decode_with_mode(&wire, true).is_err());
    }
}
