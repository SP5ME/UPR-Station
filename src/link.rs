//! Bounded AX.25 connected-mode link engine, independent of sockets and UI.
use crate::ax25::{Call, Frame};
use std::collections::VecDeque;
pub const SESSIONS: usize = 8;
const QUEUE_LIMIT: usize = 16 * 1024;
const RX_N1: usize = 256;
const RX_WINDOW: u8 = 1;
const XID_PENDING_LIMIT: usize = 32;
const XID_PENDING_MS: u64 = 30_000;

#[derive(Clone, Copy, Debug, Default)]
struct XidParameters {
    modulo_128: Option<bool>,
    peer_rx_n1: Option<usize>,
    peer_rx_window: Option<u8>,
    t1_ms: Option<u64>,
    retries: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
struct Negotiated {
    extended: bool,
    tx_n1: usize,
    t1_ms: u64,
    retries: u8,
}

struct PendingXid {
    remote: Call,
    negotiated: Negotiated,
    expires: u64,
}

fn parse_xid(info: &[u8]) -> Option<XidParameters> {
    if info.is_empty() {
        return Some(XidParameters::default());
    }
    if info.len() < 4 || info[0] != 0x82 || info[1] != 0x80 {
        return None;
    }
    let group_len = usize::from(u16::from_be_bytes([info[2], info[3]]));
    if info.len() != group_len + 4 {
        return None;
    }
    let mut pos = 4;
    let mut previous_id = 0;
    let mut result = XidParameters::default();
    while pos + 2 <= info.len() {
        let id = info[pos];
        let len = usize::from(info[pos + 1]);
        if id <= previous_id {
            return None;
        }
        previous_id = id;
        pos += 2;
        if pos + len > info.len() {
            return None;
        }
        let value = &info[pos..pos + len];
        match id {
            2 if len == 2 => {
                // Balanced ABM (0x0100). The local link engine supports half
                // duplex only, so a full-duplex-only request is answered in
                // half duplex by xid_offer.
                let classes = u16::from_be_bytes([value[0], value[1]]);
                let half = classes & 0x2000 != 0;
                let full = classes & 0x4000 != 0;
                if classes & 0x0100 == 0 || half == full || classes & !0x6100 != 0 {
                    return None;
                }
            }
            3 if len == 3 => {
                // PI=3 is a big-endian 24-bit bit field. REJ must be common to
                // both peers; SREJ may also be advertised by the peer, but this
                // engine selects only the shared REJ recovery function.
                let functions = value[0] & 0x06;
                let modulo_8 = value[1] & 0x04 != 0;
                let modulo_128 = value[1] & 0x08 != 0;
                let mandatory_bits =
                    value[0] & 0x80 != 0 && value[1] & 0xa0 == 0xa0 && value[2] & 0x02 != 0;
                if functions == 0
                    || !mandatory_bits
                    || modulo_8 == modulo_128
                    || value[0] & !0x86 != 0
                    || value[1] & !0xac != 0
                    || value[2] & !0x62 != 0
                {
                    return None;
                }
                result.modulo_128 = Some(modulo_128);
            }
            6 if len == 2 => {
                let bits = usize::from(u16::from_be_bytes([value[0], value[1]]));
                if bits == 0 || bits % 8 != 0 {
                    return None;
                }
                result.peer_rx_n1 = Some(bits / 8);
            }
            8 if len == 1 => {
                if value[0] == 0 || value[0] > 127 {
                    return None;
                }
                result.peer_rx_window = Some(value[0]);
            }
            9 if len == 2 => {
                let t1 = u64::from(u16::from_be_bytes([value[0], value[1]]));
                if t1 == 0 {
                    return None;
                }
                result.t1_ms = Some(t1);
            }
            10 if len == 1 => {
                if value[0] == 0 {
                    return None;
                }
                result.retries = Some(value[0]);
            }
            2 | 3 | 6 | 8 | 9 | 10 => return None,
            _ => {}
        }
        pos += len;
    }
    let extended = result.modulo_128.unwrap_or(false);
    let window_valid = result
        .peer_rx_window
        .is_none_or(|window| window <= if extended { 127 } else { 7 });
    (pos == info.len() && window_valid).then_some(result)
}

fn xid_offer(modulo_128: bool, t1_ms: u64, retries: u8) -> Vec<u8> {
    let mode = if modulo_128 { 0xa8 } else { 0xa4 };
    let rx_bits = u16::try_from(RX_N1 * 8).unwrap_or(u16::MAX);
    let mut parameters = vec![
        2,
        2,
        0x21,
        0x00, // Half-duplex ABM.
        3,
        3,
        0x82,
        mode,
        0x02, // Implicit REJ and one sequence mode.
        6,
        2,
        (rx_bits >> 8) as u8,
        rx_bits as u8, // Receive N1 in bits.
        8,
        1,
        RX_WINDOW, // Receive window; the engine currently supports 1.
    ];
    // PI=9 is a two-octet millisecond value; clamp larger local timers to the
    // largest interoperable value and use that same value for the session.
    let t1 = t1_ms.min(u64::from(u16::MAX)) as u16;
    parameters.extend([9, 2, (t1 >> 8) as u8, t1 as u8]);
    parameters.extend([10, 1, retries]);
    let group_len = u16::try_from(parameters.len()).unwrap_or(u16::MAX);
    let mut info = vec![0x82, 0x80, (group_len >> 8) as u8, group_len as u8];
    info.extend(parameters);
    info
}

fn negotiated_from_peer(
    local_paclen: usize,
    local_t1: u64,
    local_retries: u8,
    peer: XidParameters,
) -> Negotiated {
    let local_t1 = local_t1.min(u64::from(u16::MAX));
    let remote_t1 = peer.t1_ms.unwrap_or(local_t1);
    let remote_retries = peer.retries.unwrap_or(local_retries);
    // The engine has one transmit slot, so a peer's larger advertised
    // receive window cannot increase our window beyond one outstanding frame.
    let _tx_window = peer.peer_rx_window.unwrap_or(RX_WINDOW).min(RX_WINDOW);
    Negotiated {
        extended: peer.modulo_128.unwrap_or(false),
        tx_n1: local_paclen.min(peer.peer_rx_n1.unwrap_or(RX_N1)).max(1),
        t1_ms: local_t1.max(remote_t1),
        retries: local_retries.max(remote_retries),
    }
}
fn apply_negotiated(session: &mut Session, negotiated: Negotiated) {
    session.extended = negotiated.extended;
    session.tx_n1 = negotiated.tx_n1;
    session.t1_ms = negotiated.t1_ms;
    session.retries_limit = negotiated.retries;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Negotiating,
    Connecting,
    Connected,
    Disconnecting,
}
pub struct Session {
    pub incoming: bool,
    pub interface: usize,
    pub remote: Call,
    pub state: State,
    /// Sequence-number mode selected by the SABM/SABME handshake.
    pub extended: bool,
    xid_attempted: bool,
    /// A SABME attempt may fall back to SABM at most once.
    fallback_attempted: bool,
    // Outbound limits selected by XID. The receive side remains bounded by
    // the implementation-wide RX_N1/RX_WINDOW constants.
    tx_n1: usize,
    t1_ms: u64,
    retries_limit: u8,
    path: Vec<Call>,
    vs: u8,
    vr: u8,
    pending: Option<(u8, Vec<u8>)>,
    queue: VecDeque<Vec<u8>>,
    busy: bool,
    reject: bool,
    polling: bool,
    deadline: u64,
    retries: u8,
}
impl Session {
    pub fn buffered(&self) -> usize {
        self.queue.iter().map(Vec::len).sum::<usize>()
            + self.pending.as_ref().map_or(0, |(_, v)| v.len())
    }
}
#[derive(Debug)]
pub enum Event {
    Established(usize),
    Transmit(Frame),
    /// A transmission explicitly routed through one configured TNC interface.
    TransmitOn(usize, Frame),
    Status(usize, String),
    /// Local UI information; no link-state transition or radio transmission.
    Notice(usize, String),
    Data(usize, Vec<u8>),
}
pub struct Engine {
    pub local: Call,
    pub sessions: [Option<Session>; SESSIONS],
    pub accept: bool,
    /// Upper bound for simultaneous incoming links; outgoing links are unaffected.
    pub incoming_limit: usize,
    pub t1: u64,
    pub t3: u64,
    pub n2: u8,
    pub paclen: usize,
    /// Initiate and accept modulo-128 links with SABME.
    pub modulo_128: bool,
    /// Initiate XID parameter negotiation after link establishment.
    pub autonegotiation: bool,
    pending_xids: VecDeque<PendingXid>,
}
impl Engine {
    /// Maximum information field accepted for the selected session.
    /// Protocols that add their own envelope must subtract that envelope
    /// before choosing a payload size.
    pub fn tx_info_limit(&self, id: usize) -> Option<usize> {
        self.sessions
            .get(id)
            .and_then(Option::as_ref)
            .map(|session| session.tx_n1.max(1))
    }
    pub fn new(local: Call) -> Self {
        Self {
            local,
            sessions: std::array::from_fn(|_| None),
            accept: true,
            incoming_limit: SESSIONS,
            t1: 5000,
            t3: 180000,
            n2: 10,
            paclen: 128,
            modulo_128: false,
            autonegotiation: false,
            pending_xids: VecDeque::new(),
        }
    }
    fn frame(&self, s: &Session, control: u16, command: bool, info: Vec<u8>) -> Frame {
        Frame {
            destination: s.remote.clone(),
            source: self.local.clone(),
            path: s.path.iter().cloned().map(|c| (c, false)).collect(),
            command,
            response: !command,
            control,
            extended: s.extended,
            pid: if control & 1 == 0 { Some(0xf0) } else { None },
            info,
        }
    }
    fn tx(&self, s: &Session, c: u16, command: bool, out: &mut Vec<Event>) {
        out.push(Event::Transmit(self.frame(s, c, command, vec![])));
    }
    fn tx_info(&self, s: &Session, c: u16, command: bool, info: Vec<u8>, out: &mut Vec<Event>) {
        out.push(Event::Transmit(self.frame(s, c, command, info)));
    }
    fn make(
        &self,
        remote: Call,
        path: Vec<Call>,
        state: State,
        now: u64,
        interface: usize,
        extended: bool,
    ) -> Session {
        Session {
            incoming: false,
            interface,
            remote,
            path,
            state,
            extended,
            xid_attempted: false,
            fallback_attempted: false,
            tx_n1: self.paclen,
            t1_ms: self.t1,
            retries_limit: self.n2,
            vs: 0,
            vr: 0,
            pending: None,
            queue: VecDeque::new(),
            busy: false,
            reject: false,
            polling: false,
            deadline: now + self.t1,
            retries: 0,
        }
    }
    fn take_pending_xid(&mut self, remote: &Call, now: u64) -> Option<PendingXid> {
        self.pending_xids.retain(|pending| pending.expires > now);
        let index = self
            .pending_xids
            .iter()
            .position(|pending| &pending.remote == remote)?;
        self.pending_xids.remove(index)
    }
    pub fn connect(
        &mut self,
        id: usize,
        remote: Call,
        path: Vec<Call>,
        now: u64,
    ) -> Result<Vec<Event>, String> {
        self.connect_on(id, remote, path, now, 0)
    }
    pub fn connect_on(
        &mut self,
        id: usize,
        remote: Call,
        path: Vec<Call>,
        now: u64,
        interface: usize,
    ) -> Result<Vec<Event>, String> {
        if id >= SESSIONS || self.sessions[id].is_some() {
            return Err("Session slot is occupied/invalid".into());
        }
        if remote == self.local || path.len() > 8 {
            return Err("Invalid destination/path".into());
        }
        if self.sessions.iter().flatten().any(|s| s.remote == remote) {
            return Err("This station already has a session".into());
        }
        // SABME selects modulo 128 during link establishment. XID is a
        // separate parameter exchange and must not switch an already active
        // modulo-8 link to modulo 128.
        let extended = self.modulo_128;
        let s = self.make(remote, path, State::Connecting, now, interface, extended);
        let mut out = vec![];
        if extended {
            self.tx(&s, 0x7f, true, &mut out);
            out.push(Event::Status(
                id,
                if self.autonegotiation {
                    "Connecting (modulo 128; XID follows UA)"
                } else {
                    "Connecting (modulo 128)"
                }
                .into(),
            ));
        } else if self.autonegotiation {
            self.tx(&s, 0x3f, true, &mut out);
            out.push(Event::Status(
                id,
                "Connecting (modulo 8; XID follows UA)".into(),
            ));
        } else {
            self.tx(&s, 0x3f, true, &mut out);
            out.push(Event::Status(id, "Connecting".into()));
        }
        self.sessions[id] = Some(s);
        Ok(out)
    }
    /// User-facing connect selection shared by CLI and TUI. Existing sessions emit no RF.
    pub fn connect_selected(
        &mut self,
        selected: usize,
        remote: Call,
        path: Vec<Call>,
        now: u64,
    ) -> Result<(usize, Vec<Event>), String> {
        self.connect_selected_on(selected, remote, path, now, 0)
    }
    pub fn connect_selected_on(
        &mut self,
        selected: usize,
        remote: Call,
        path: Vec<Call>,
        now: u64,
        interface: usize,
    ) -> Result<(usize, Vec<Event>), String> {
        if let Some(id) = self
            .sessions
            .iter()
            .position(|s| s.as_ref().is_some_and(|s| s.remote == remote))
        {
            return Ok((
                id,
                vec![Event::Notice(
                    id,
                    format!(
                        "Session with {remote} already exists. Switched to session {}.",
                        id + 1
                    ),
                )],
            ));
        }
        let id = if selected < SESSIONS && self.sessions[selected].is_none() {
            selected
        } else {
            self.sessions
                .iter()
                .position(Option::is_none)
                .ok_or("All eight sessions are occupied")?
        };
        self.connect_on(id, remote, path, now, interface)
            .map(|events| (id, events))
    }
    pub fn disconnect(&mut self, id: usize, now: u64) -> Result<Vec<Event>, String> {
        let mut s = self
            .sessions
            .get_mut(id)
            .and_then(Option::take)
            .ok_or("No session")?;
        if s.state == State::Connecting && !s.extended {
            // No link exists yet. Cancel the local attempt without emitting a
            // DISC exchange or leaving a retry timer active.
            return Ok(vec![Event::Status(id, "Connection cancelled".into())]);
        }
        s.state = State::Disconnecting;
        s.pending = None;
        s.queue.clear();
        s.retries = 0;
        s.deadline = now + s.t1_ms;
        let mut out = vec![];
        self.tx(&s, 0x53, true, &mut out);
        self.sessions[id] = Some(s);
        Ok(out)
    }
    pub fn send(&mut self, id: usize, data: &[u8], now: u64) -> Result<Vec<Event>, String> {
        let s = self
            .sessions
            .get(id)
            .and_then(Option::as_ref)
            .ok_or("No session")?;
        let link_established = s.state == State::Connected
            || s.state == State::Negotiating
            || (s.state == State::Connecting && s.extended && s.xid_attempted);
        if !link_established {
            return Err("Session is not connected".into());
        }
        if s.buffered() + data.len() > QUEUE_LIMIT {
            return Err("TX queue full (16 KiB); wait for acknowledgements".into());
        }
        let mut s = self.sessions[id].take().unwrap();
        for chunk in data.chunks(s.tx_n1.max(1)) {
            s.queue.push_back(chunk.to_vec());
        }
        let mut out = vec![];
        self.flush(&mut s, now, &mut out);
        self.sessions[id] = Some(s);
        Ok(out)
    }
    pub fn send_burst(
        &mut self,
        id: usize,
        frames: &[Vec<u8>],
        now: u64,
    ) -> Result<Vec<Event>, String> {
        let s = self
            .sessions
            .get(id)
            .and_then(Option::as_ref)
            .ok_or("No session")?;
        let link_established = s.state == State::Connected
            || s.state == State::Negotiating
            || (s.state == State::Connecting && s.extended && s.xid_attempted);
        if !link_established || s.pending.is_some() || s.busy || s.polling || !s.queue.is_empty() {
            return Err("Session is not ready for burst transmission".into());
        }
        let mut s = self.sessions[id].take().unwrap();
        let modulo = if s.extended { 128 } else { 8 };
        let mut out = Vec::with_capacity(frames.len());
        for data in frames {
            if data.len() > s.tx_n1.max(1) {
                self.sessions[id] = Some(s);
                return Err("Burst frame exceeds negotiated AX.25 size".into());
            }
            let seq = s.vs;
            s.vs = (s.vs + 1) % modulo;
            out.push(Event::Transmit(self.frame(
                &s,
                if s.extended {
                    (u16::from(seq) << 1) | (u16::from(s.vr) << 9)
                } else {
                    (u16::from(s.vr) << 5) | (u16::from(seq) << 1)
                },
                true,
                data.clone(),
            )));
        }
        s.deadline = now + s.t1_ms;
        self.sessions[id] = Some(s);
        Ok(out)
    }
    fn flush(&self, s: &mut Session, now: u64, out: &mut Vec<Event>) {
        if s.state != State::Connected || s.pending.is_some() || s.busy || s.polling {
            return;
        }
        if let Some(data) = s.queue.pop_front() {
            let seq = s.vs;
            let modulo = if s.extended { 128 } else { 8 };
            s.vs = (s.vs + 1) % modulo;
            out.push(Event::Transmit(self.frame(
                s,
                if s.extended {
                    (u16::from(seq) << 1) | (u16::from(s.vr) << 9)
                } else {
                    (u16::from(s.vr) << 5) | (u16::from(seq) << 1)
                },
                true,
                data.clone(),
            )));
            s.pending = Some((seq, data));
            s.deadline = now + s.t1_ms;
        }
    }
    fn retransmit(&self, s: &mut Session, now: u64, out: &mut Vec<Event>) {
        if !s.busy
            && let Some((seq, data)) = &s.pending
        {
            out.push(Event::Transmit(self.frame(
                s,
                if s.extended {
                    (u16::from(*seq) << 1) | (u16::from(s.vr) << 9)
                } else {
                    (u16::from(s.vr) << 5) | (u16::from(*seq) << 1)
                },
                true,
                data.clone(),
            )));
            s.deadline = now + s.t1_ms;
        }
    }
    fn retransmit_requested(&self, s: &mut Session, nr: u8, now: u64, out: &mut Vec<Event>) {
        if s.pending.as_ref().is_some_and(|(seq, _)| *seq == nr) {
            self.retransmit(s, now, out);
        }
    }
    fn ack(&self, s: &mut Session, nr: u8, now: u64) -> bool {
        if nr == s.vs {
            if s.pending.take().is_some() {
                s.retries = 0;
                if !s.polling {
                    s.deadline = now + self.t3;
                }
            }
            true
        } else {
            s.pending.as_ref().is_some_and(|(seq, _)| *seq == nr)
        }
    }
    pub fn receive(&mut self, f: Frame, now: u64) -> Vec<Event> {
        self.receive_on(f, now, 0)
    }
    pub fn extended_for(&self, remote: &Call) -> bool {
        self.sessions
            .iter()
            .flatten()
            .find(|session| &session.remote == remote)
            .is_some_and(|session| session.extended)
    }
    pub fn receive_on(&mut self, f: Frame, now: u64, interface: usize) -> Vec<Event> {
        let mut out = vec![];
        if f.destination != self.local
            || f.source == self.local
            || f.path.iter().any(|(_, h)| !*h)
            || f.command == f.response
        {
            return out;
        }
        let command = f.command;
        let pf = f.pf();
        let u = f.control & 0x00ef;
        let existing = self
            .sessions
            .iter()
            .position(|s| s.as_ref().is_some_and(|s| s.remote == f.source));
        if existing.is_some_and(|id| {
            self.sessions[id]
                .as_ref()
                .is_some_and(|s| s.interface != interface)
        }) {
            return out;
        }
        let reverse = f
            .path
            .iter()
            .rev()
            .map(|(c, _)| c.clone())
            .collect::<Vec<_>>();
        if u == 0xaf {
            if command {
                let Some(request) = parse_xid(&f.info) else {
                    return out;
                };
                let existing_mode = existing
                    .and_then(|id| self.sessions[id].as_ref())
                    .map(|session| session.extended);
                let mut negotiated = negotiated_from_peer(self.paclen, self.t1, self.n2, request);
                negotiated.extended = existing_mode.unwrap_or(false)
                    || (self.modulo_128 && negotiated.extended);
                if let Some(id) = existing {
                    // XID on an active link reports its current mode; it does
                    // not authorize a unilateral change of control format.
                    negotiated.extended = self.sessions[id]
                        .as_ref()
                        .is_some_and(|session| session.extended);
                }
                let s = self.make(
                    f.source.clone(),
                    reverse,
                    State::Connecting,
                    now,
                    interface,
                    negotiated.extended,
                );
                self.tx_info(
                    &s,
                    0xaf | if pf { 0x10 } else { 0 },
                    false,
                    xid_offer(negotiated.extended, negotiated.t1_ms, negotiated.retries),
                    &mut out,
                );
                // A pre-connect XID may supply timers/N1 for the subsequent
                // SABM(E), but it never selects the control-field format.
                // Once a session already exists, there is no pending link
                // transition to attach this response to.
                if existing.is_none() {
                    self.pending_xids.retain(|pending| pending.expires > now);
                    self.pending_xids
                        .retain(|pending| pending.remote != f.source);
                    if self.pending_xids.len() == XID_PENDING_LIMIT {
                        self.pending_xids.pop_front();
                    }
                    self.pending_xids.push_back(PendingXid {
                        remote: f.source,
                        negotiated,
                        expires: now.saturating_add(XID_PENDING_MS),
                    });
                }
            } else if let Some(id) = existing
                && self.sessions[id]
                    .as_ref()
                    .is_some_and(|session| session.state == State::Negotiating)
                && pf
            {
                let mut session = self.sessions[id].take().unwrap();
                if let Some(response) = parse_xid(&f.info) {
                    let mut negotiated = negotiated_from_peer(self.paclen, self.t1, self.n2, response);
                    let active_mode = session.extended;
                    let mode_mismatch = negotiated.extended != active_mode;
                    negotiated.extended = active_mode;
                    apply_negotiated(&mut session, negotiated);
                    session.retries = 0;
                    session.state = State::Connected;
                    session.deadline = now + self.t3;
                    out.push(Event::Status(
                        id,
                        if mode_mismatch {
                            "XID mode differs from active link; keeping current format"
                        } else if active_mode {
                            "XID parameters accepted; link remains modulo 128"
                        } else {
                            "XID parameters accepted; link remains modulo 8"
                        }
                        .into(),
                    ));
                    out.push(Event::Established(id));
                    self.flush(&mut session, now, &mut out);
                } else {
                    session.state = State::Connected;
                    session.deadline = now + self.t3;
                    out.push(Event::Established(id));
                    out.push(Event::Status(
                        id,
                        "Invalid XID response; connected in modulo 8".into(),
                    ));
                    self.flush(&mut session, now, &mut out);
                }
                self.sessions[id] = Some(session);
            }
            return out;
        }
        if command && u == 0x6f {
            if !self.modulo_128 {
                let s = self.make(f.source, reverse, State::Connecting, now, interface, false);
                self.tx(&s, 0x0f | if pf { 16 } else { 0 }, false, &mut out);
                return out;
            }
            let slot = existing.or_else(|| {
                if self.accept
                    && self
                        .sessions
                        .iter()
                        .flatten()
                        .filter(|s| s.incoming)
                        .count()
                        < self.incoming_limit
                {
                    self.sessions.iter().position(Option::is_none)
                } else {
                    None
                }
            });
            if f.pid.is_some() || !f.info.is_empty() {
                let s = self.make(f.source, reverse, State::Connecting, now, interface, true);
                self.tx(&s, 0x0f | if pf { 16 } else { 0 }, false, &mut out);
            } else if let Some(id) = slot {
                let pending = self.take_pending_xid(&f.source, now);
                let incoming =
                    existing.is_none() || self.sessions[id].as_ref().is_some_and(|s| s.incoming);
                let mut s = self.make(f.source, reverse, State::Connected, now, interface, true);
                if let Some(pending) = pending {
                    let mut negotiated = pending.negotiated;
                    negotiated.extended = true;
                    apply_negotiated(&mut s, negotiated);
                }
                s.incoming = incoming;
                s.deadline = now + self.t3;
                self.tx(&s, 0x63 | if pf { 0x10 } else { 0 }, false, &mut out);
                out.push(Event::Status(
                    id,
                    "Connected (SABME; modulo 128; sequence state reset)".into(),
                ));
                self.sessions[id] = Some(s);
                out.push(Event::Established(id));
            } else {
                let s = self.make(f.source, reverse, State::Connecting, now, interface, true);
                self.tx(&s, 0x0f | if pf { 16 } else { 0 }, false, &mut out);
            }
            return out;
        }
        if command && u == 0x2f {
            let slot = existing.or_else(|| {
                if self.accept
                    && self
                        .sessions
                        .iter()
                        .flatten()
                        .filter(|s| s.incoming)
                        .count()
                        < self.incoming_limit
                {
                    self.sessions.iter().position(Option::is_none)
                } else {
                    None
                }
            });
            if let Some(id) = slot {
                let pending = self.take_pending_xid(&f.source, now);
                let incoming =
                    existing.is_none() || self.sessions[id].as_ref().is_some_and(|s| s.incoming);
                let mut s = self.make(f.source, reverse, State::Connected, now, interface, false);
                if let Some(pending) = pending {
                    let mut negotiated = pending.negotiated;
                    negotiated.extended = false;
                    apply_negotiated(&mut s, negotiated);
                }
                s.incoming = incoming;
                s.deadline = now + self.t3;
                self.tx(&s, 0x63 | if pf { 16 } else { 0 }, false, &mut out);
                out.push(Event::Status(
                    id,
                    "Connected (SABM; sequence state reset)".into(),
                ));
                self.sessions[id] = Some(s);
                out.push(Event::Established(id));
            } else {
                let s = self.make(f.source, reverse, State::Connecting, now, interface, false);
                self.tx(&s, 0x0f | if pf { 16 } else { 0 }, false, &mut out);
            }
            return out;
        }
        let Some(id) = existing else {
            if command && (u == 0x43 || (pf && u != 3)) {
                let s = self.make(f.source, reverse, State::Connecting, now, interface, false);
                self.tx(&s, 0x0f | if pf { 16 } else { 0 }, false, &mut out);
            }
            return out;
        };
        let mut s = self.sessions[id].take().unwrap();
        if s.state == State::Negotiating && !command && (u == 0x0f || u == 0x87) {
            // XID is optional. A FRMR is a valid compatibility response from
            // a pre-v2.2 peer, but ending this negotiation is local policy;
            // it is not a mode reset and must not change the active format.
            s.state = State::Connected;
            s.deadline = now + self.t3;
            s.retries = 0;
            out.push(Event::Status(
                id,
                if u == 0x87 {
                    "XID FRMR; local compatibility policy keeps active link format"
                } else {
                    "XID rejected; keeping active link format"
                }
                .into(),
            ));
            out.push(Event::Established(id));
            self.flush(&mut s, now, &mut out);
            self.sessions[id] = Some(s);
            return out;
        }
        if s.state == State::Connecting
            && s.extended
            && !s.fallback_attempted
            && !command
            && (u == 0x0f || u == 0x87)
        {
            // DM explicitly rejects SABME. AX.25 v2.2 also allows an older
            // peer to answer SABME with FRMR. Retrying with SABM is a local
            // compatibility policy, not a mandatory state-machine transition.
            // UA has no SABM/SABME request identifier, so a delayed UA remains
            // inherently ambiguous; it must not enable modulo 128 here.
            let reason = if u == 0x87 { "FRMR" } else { "DM" };
            s.extended = false;
            s.fallback_attempted = true;
            s.vs = 0;
            s.vr = 0;
            // The old SABME attempt is reset, but locally queued messages are
            // retained. Any pending frame belonged to the reset attempt and
            // is retransmitted from the preserved queue after SABM/UA.
            s.pending = None;
            s.retries = 0;
            s.t1_ms = self.t1;
            s.retries_limit = self.n2;
            s.deadline = now + s.t1_ms;
            self.tx(&s, 0x3f, true, &mut out);
            out.push(Event::Status(
                id,
                format!(
                    "SABME odrzucone ({reason}) — próba połączenia w modulo 8"
                ),
            ));
            self.sessions[id] = Some(s);
            return out;
        }
        if command && u == 0x43 {
            self.tx(&s, 0x63 | if pf { 16 } else { 0 }, false, &mut out);
            out.push(Event::Status(id, "Disconnected by peer".into()));
            return out;
        }
        if !command && u == 0x0f {
            out.push(Event::Status(id, "Disconnected/refused (DM)".into()));
            return out;
        }
        if !command && u == 0x63 && pf {
            match s.state {
                State::Negotiating => {}
                State::Connecting if self.autonegotiation && !s.xid_attempted => {
                    // XID negotiates parameters after UA. It never changes
                    // the active SABM/SABME control-field format.
                    s.state = State::Negotiating;
                    s.xid_attempted = true;
                    s.deadline = now + s.t1_ms;
                    s.retries = 0;
                    self.tx_info(
                        &s,
                        0xbf,
                        true,
                        xid_offer(s.extended, self.t1, self.n2),
                        &mut out,
                    );
                    out.push(Event::Status(
                        id,
                        if s.extended {
                            "Connected in modulo 128; negotiating AX.25 parameters (XID)"
                        } else {
                            "Connected in modulo 8; negotiating AX.25 parameters (XID)"
                        }
                        .into(),
                    ));
                }
                State::Connecting => {
                    s.state = State::Connected;
                    s.deadline = now + self.t3;
                    s.retries = 0;
                    let fallback = s.fallback_attempted && !s.extended;
                    out.push(Event::Status(
                        id,
                        if fallback {
                            "Połączono w modulo 8 (fallback z modulo 128)"
                        } else if s.extended {
                            "Connected (modulo 128)"
                        } else {
                            "Connected"
                        }
                        .into(),
                    ));
                    out.push(Event::Established(id));
                }
                State::Disconnecting => {
                    out.push(Event::Status(id, "Disconnected".into()));
                    return out;
                }
                State::Connected => {}
            }
            self.flush(&mut s, now, &mut out);
        } else if matches!(s.state, State::Connected | State::Negotiating) {
            if f.control & 1 == 0 || f.control & 3 == 1 {
                let nr = if s.extended {
                    (f.control >> 9) as u8
                } else {
                    (f.control >> 5) as u8
                };
                let srej_without_ack = f.control & 3 == 1 && f.control & 15 == 13 && !pf;
                if !srej_without_ack && !self.ack(&mut s, nr, now) {
                    out.push(Event::Status(
                        id,
                        "Invalid N(R); resetting link, queued data discarded".into(),
                    ));
                    let incoming = s.incoming;
                    s = self.make(
                        s.remote,
                        s.path,
                        State::Connecting,
                        now,
                        s.interface,
                        s.extended,
                    );
                    s.incoming = incoming;
                    self.tx(&s, if s.extended { 0x7f } else { 0x3f }, true, &mut out);
                    self.sessions[id] = Some(s);
                    return out;
                }
                if f.control & 1 == 0 {
                    let ns = if s.extended {
                        ((f.control >> 1) & 0x7f) as u8
                    } else {
                        ((f.control >> 1) & 7) as u8
                    };
                    if ns == s.vr {
                        let modulo = if s.extended { 128 } else { 8 };
                        s.vr = (s.vr + 1) % modulo;
                        s.reject = false;
                        if f.pid == Some(0xf0) {
                            out.push(Event::Data(id, f.info));
                        } else {
                            out.push(Event::Status(
                                id,
                                "Non-text PID received; payload not displayed".into(),
                            ));
                        }
                        self.tx(
                            &s,
                            if s.extended {
                                1 | (u16::from(s.vr) << 9) | if command && pf { 0x0100 } else { 0 }
                            } else {
                                1 | (u16::from(s.vr) << 5) | if command && pf { 16 } else { 0 }
                            },
                            false,
                            &mut out,
                        );
                    } else if !s.reject || command && pf {
                        s.reject = true;
                        self.tx(
                            &s,
                            if s.extended {
                                9 | (u16::from(s.vr) << 9) | if command && pf { 0x0100 } else { 0 }
                            } else {
                                9 | (u16::from(s.vr) << 5) | if command && pf { 16 } else { 0 }
                            },
                            false,
                            &mut out,
                        );
                    }
                } else {
                    let kind = f.control & 15;
                    s.busy = kind == 5;
                    if s.busy && !s.polling {
                        s.deadline = now + s.t1_ms;
                    }
                    if command && pf {
                        self.tx(
                            &s,
                            if s.extended {
                                1 | (u16::from(s.vr) << 9) | 0x0100
                            } else {
                                1 | (u16::from(s.vr) << 5) | 16
                            },
                            false,
                            &mut out,
                        );
                    }
                    if !command && pf && s.polling {
                        s.polling = false;
                        s.deadline = now
                            + if s.busy || s.pending.is_some() {
                                s.t1_ms
                            } else {
                                self.t3
                            };
                        if !s.busy && s.pending.is_none() {
                            s.retries = 0;
                        }
                        if kind == 13 {
                            self.retransmit_requested(&mut s, nr, now, &mut out);
                        } else {
                            self.retransmit(&mut s, now, &mut out);
                        }
                    } else if kind == 9 || kind == 13 {
                        if kind == 13 {
                            self.retransmit_requested(&mut s, nr, now, &mut out);
                        } else {
                            self.retransmit(&mut s, now, &mut out);
                        }
                    }
                }
                if s.state == State::Connected && !s.polling && s.pending.is_none() && !s.busy {
                    s.deadline = now + self.t3;
                }
            } else if !command && u == 0x87 {
                out.push(Event::Status(
                    id,
                    "FRMR received; resetting link, queued data discarded".into(),
                ));
                let incoming = s.incoming;
                s = self.make(
                    s.remote,
                    s.path,
                    State::Connecting,
                    now,
                    s.interface,
                    s.extended,
                );
                s.incoming = incoming;
                self.tx(&s, if s.extended { 0x7f } else { 0x3f }, true, &mut out);
            }
            self.flush(&mut s, now, &mut out);
        }
        self.sessions[id] = Some(s);
        out
    }
    pub fn tick(&mut self, now: u64) -> Vec<Event> {
        let mut out = vec![];
        for id in 0..SESSIONS {
            let Some(mut s) = self.sessions[id].take() else {
                continue;
            };
            if now >= s.deadline {
                if s.retries >= s.retries_limit {
                    out.push(Event::Status(
                        id,
                        "Retry limit reached; session closed, unsent data discarded".into(),
                    ));
                    continue;
                }
                s.retries += 1;
                s.deadline = now + s.t1_ms;
                match s.state {
                    State::Negotiating => {
                        // XID is optional. Keep the already-established link
                        // mode if the peer is silent.
                        s.state = State::Connected;
                        s.retries = 0;
                        s.deadline = now + self.t3;
                        out.push(Event::Status(
                            id,
                            if s.extended {
                                "XID timeout; keeping modulo 128 link"
                            } else {
                                "XID timeout; keeping modulo 8 link"
                            }
                            .into(),
                        ));
                        out.push(Event::Established(id));
                        self.flush(&mut s, now, &mut out);
                    }
                    State::Connecting => {
                        self.tx(&s, if s.extended { 0x7f } else { 0x3f }, true, &mut out)
                    }
                    State::Disconnecting => self.tx(&s, 0x53, true, &mut out),
                    State::Connected => {
                        s.polling = true;
                        self.tx(
                            &s,
                            if s.extended {
                                1 | (u16::from(s.vr) << 9) | 0x0100
                            } else {
                                1 | (u16::from(s.vr) << 5) | 16
                            },
                            true,
                            &mut out,
                        );
                    }
                }
            }
            self.sessions[id] = Some(s);
        }
        out
    }
    pub fn transport_lost(&mut self) -> Vec<Event> {
        let mut out = vec![];
        for (id, s) in self.sessions.iter_mut().enumerate() {
            if s.take().is_some() {
                out.push(Event::Status(
                    id,
                    "TCP lost; session and queued data cleared".into(),
                ));
            }
        }
        out
    }
    pub fn interface_lost(&mut self, interface: usize) -> Vec<Event> {
        let mut out = Vec::new();
        for (id, session) in self.sessions.iter_mut().enumerate() {
            if session.as_ref().is_some_and(|s| s.interface == interface) {
                *session = None;
                out.push(Event::Status(
                    id,
                    "TCP lost; session and queued data cleared".into(),
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(control: u16, extended: bool, command: bool, info: Vec<u8>) -> Frame {
        Frame {
            destination: "LOCAL".parse().unwrap(),
            source: "REMOTE".parse().unwrap(),
            path: vec![],
            command,
            response: !command,
            control,
            extended,
            pid: if control & 1 == 0 { Some(0xf0) } else { None },
            info,
        }
    }

    fn engine(modulo_128: bool, xid: bool) -> (Engine, Call) {
        let remote: Call = "REMOTE".parse().unwrap();
        let mut engine = Engine::new("LOCAL".parse().unwrap());
        engine.modulo_128 = modulo_128;
        engine.autonegotiation = xid;
        engine.t1 = 100;
        engine.n2 = 2;
        (engine, remote)
    }

    fn establish(engine: &mut Engine, remote: &Call, now: u64) -> Vec<Event> {
        let ua = Frame {
            destination: "LOCAL".parse().unwrap(),
            source: remote.clone(),
            path: vec![],
            command: false,
            response: true,
            control: 0x73,
            extended: false,
            pid: None,
            info: vec![],
        };
        engine.receive_on(ua, now, 0)
    }

    #[test]
    fn sabme_uses_modulo_128_and_xid_does_not_change_active_mode() {
        let (mut engine, remote) = engine(true, true);
        let first = engine.connect(0, remote.clone(), vec![], 0).unwrap();
        assert!(matches!(first.first(), Some(Event::Transmit(f)) if f.control == 0x7f));
        let after_ua = establish(&mut engine, &remote, 1);
        assert!(after_ua.iter().any(|event| matches!(event, Event::Transmit(f) if f.control == 0xbf)));
        let response = frame(0xbf, false, false, xid_offer(false, 100, 2));
        let result = engine.receive_on(response, 2, 0);
        assert!(!result.iter().any(|event| matches!(event, Event::Transmit(f) if f.control == 0x7f)));
        assert!(engine.sessions[0].as_ref().is_some_and(|session| session.extended));
    }

    #[test]
    fn incoming_i_is_processed_while_xid_is_negotiating_and_acknowledged() {
        let (mut engine, remote) = engine(false, true);
        engine.connect(0, remote.clone(), vec![], 0).unwrap();
        let after_ua = establish(&mut engine, &remote, 1);
        assert!(after_ua.iter().any(|event| matches!(event, Event::Transmit(f) if f.control == 0xbf)));
        let result = engine.receive_on(frame(0, false, true, b"data".to_vec()), 2, 0);
        assert!(result.iter().any(|event| matches!(event, Event::Data(0, data) if data == b"data")));
        assert!(result.iter().any(|event| matches!(event, Event::Transmit(f) if f.control == 0x21)));
        assert!(engine.sessions[0].as_ref().is_some_and(|session| session.state == State::Negotiating));
    }

    #[test]
    fn dm_falls_back_to_sabm_and_late_ua_cannot_enable_extended_mode() {
        let (mut engine, remote) = engine(true, false);
        engine.connect(0, remote.clone(), vec![], 0).unwrap();
        engine.sessions[0]
            .as_mut()
            .unwrap()
            .queue
            .push_back(b"queued".to_vec());
        let dm = Frame {
            destination: "LOCAL".parse().unwrap(),
            source: remote.clone(),
            path: vec![],
            command: false,
            response: true,
            control: 0x1f,
            extended: false,
            pid: None,
            info: vec![],
        };
        let result = engine.receive_on(dm, 1, 0);
        assert!(result.iter().any(|event| matches!(event, Event::Transmit(f) if f.control == 0x3f)));
        assert!(result.iter().any(|event| matches!(event, Event::Status(0, message) if message == "SABME odrzucone (DM) — próba połączenia w modulo 8")));
        assert!(engine.sessions[0].as_ref().is_some_and(|session| !session.extended));
        assert_eq!(engine.sessions[0].as_ref().unwrap().buffered(), b"queued".len());
        let late_ua = establish(&mut engine, &remote, 2);
        assert!(late_ua.iter().any(|event| matches!(event, Event::Established(0))));
        assert!(late_ua.iter().any(|event| matches!(event, Event::Status(0, message) if message == "Połączono w modulo 8 (fallback z modulo 128)")));
        assert!(engine.sessions[0].as_ref().is_some_and(|session| !session.extended));
    }

    #[test]
    fn frmr_falls_back_to_sabm_as_local_compatibility_policy() {
        let (mut engine, remote) = engine(true, false);
        engine.connect(0, remote.clone(), vec![], 0).unwrap();
        let frmr = Frame {
            destination: "LOCAL".parse().unwrap(),
            source: remote.clone(),
            path: vec![],
            command: false,
            response: true,
            control: 0x97,
            extended: false,
            pid: None,
            info: vec![],
        };
        let result = engine.receive_on(frmr, 1, 0);
        assert!(result.iter().any(|event| matches!(event, Event::Transmit(f) if f.control == 0x3f)));
        assert!(result.iter().any(|event| matches!(event, Event::Status(0, message) if message == "SABME odrzucone (FRMR) — próba połączenia w modulo 8")));
        assert!(engine.sessions[0].as_ref().is_some_and(|session| !session.extended));
        let ua = establish(&mut engine, &remote, 2);
        assert!(ua.iter().any(|event| matches!(event, Event::Status(0, message) if message == "Połączono w modulo 8 (fallback z modulo 128)")));
        assert!(engine.sessions[0].is_some());
    }

    #[test]
    fn fallback_preserves_session_and_does_not_loop_after_sabm_failure() {
        let (mut engine, remote) = engine(true, false);
        engine.connect(0, remote.clone(), vec![], 0).unwrap();
        let id = engine.sessions[0].as_ref().unwrap().remote.clone();
        let dm = frame(0x1f, false, false, vec![]);
        let first = engine.receive_on(dm, 1, 0);
        assert!(first.iter().any(|event| matches!(event, Event::Transmit(f) if f.control == 0x3f)));
        assert_eq!(engine.sessions[0].as_ref().unwrap().remote, id);
        let second = engine.receive_on(frame(0x1f, false, false, vec![]), 2, 0);
        assert!(second.iter().all(|event| !matches!(event, Event::Transmit(f) if f.control == 0x3f)));
        let n2 = engine.n2;
        let t1 = engine.t1;
        for n in 1..=n2 {
            engine.tick(2 + u64::from(n) * t1);
        }
        assert!(engine.sessions[0].is_none());
    }

    #[test]
    fn incoming_xid_is_answered_even_when_automatic_xid_is_disabled() {
        let (mut engine, _remote) = engine(false, false);
        let xid = frame(0xaf, true, true, xid_offer(true, 100, 2));
        let result = engine.receive_on(xid, 1, 0);
        assert!(result.iter().any(|event| matches!(
            event,
            Event::Transmit(f) if f.control == 0xaf && !f.command
        )));
        assert!(engine.sessions.iter().all(Option::is_none));
    }

    #[test]
    fn sabme_timeout_does_not_fallback_after_one_t1() {
        let (mut engine, remote) = engine(true, false);
        engine.connect(0, remote, vec![], 0).unwrap();
        let retry = engine.tick(100);
        assert!(retry.iter().any(|event| matches!(
            event,
            Event::Transmit(f) if f.control == 0x7f
        )));
        assert!(!retry.iter().any(|event| matches!(
            event,
            Event::Transmit(f) if f.control == 0x3f
        )));
        assert!(engine.sessions[0].is_some());
    }
}
