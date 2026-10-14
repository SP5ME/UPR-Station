use ultimatepr::{
    ax25::Frame,
    link::{Engine, Event, State},
};
fn engine(call: &str) -> Engine {
    Engine::new(call.parse().unwrap())
}
fn frames(events: Vec<Event>) -> Vec<Frame> {
    events
        .into_iter()
        .filter_map(|e| {
            if let Event::Transmit(f) = e {
                Some(f)
            } else {
                None
            }
        })
        .collect()
}
fn one(events: Vec<Event>) -> Frame {
    let mut f = frames(events);
    assert_eq!(f.len(), 1);
    f.remove(0)
}
fn pair() -> (Engine, Engine) {
    let mut a = engine("TEST-1");
    let mut b = engine("TEST-2");
    let sabm = one(a.connect(0, b.local.clone(), vec![], 0).unwrap());
    let ua = one(b.receive(sabm, 1));
    a.receive(ua, 2);
    (a, b)
}
fn response(source: &str, destination: &str, control: u16) -> Frame {
    Frame {
        source: source.parse().unwrap(),
        destination: destination.parse().unwrap(),
        path: vec![],
        command: false,
        response: true,
        control,
        extended: false,
        pid: None,
        info: vec![],
    }
}
#[test]
fn bidirectional_text_wraparound_and_disconnect() {
    let (mut a, mut b) = pair();
    for n in 0..20 {
        let f = one(a.send(0, b"hello\r", 10 + n).unwrap());
        assert_eq!((f.control >> 1) & 7, u16::from(n as u8 % 8));
        let events = b.receive(f, 40 + n);
        assert!(
            events
                .iter()
                .any(|e| matches!(e,Event::Data(0,d) if d==b"hello\r"))
        );
        a.receive(one(events), 70 + n);
        assert_eq!(a.sessions[0].as_ref().unwrap().buffered(), 0);
    }
    let f = one(b.send(0, b"reply\r", 100).unwrap());
    let ack = one(a.receive(f, 101));
    b.receive(ack, 102);
    let disc = one(a.disconnect(0, 110).unwrap());
    let ua = one(b.receive(disc, 111));
    a.receive(ua, 112);
    assert!(a.sessions[0].is_none() && b.sessions[0].is_none());
}
#[test]
fn lost_ack_poll_does_not_duplicate_data() {
    let (mut a, mut b) = pair();
    let f = one(a.send(0, b"once", 10).unwrap());
    b.receive(f.clone(), 11); // Drop ACK.
    let duplicate = b.receive(f, 12);
    assert!(!duplicate.iter().any(|e| matches!(e, Event::Data(..))));
    let poll = one(a.tick(5010));
    assert_eq!(poll.control, 0x11);
    let final_ack = one(b.receive(poll, 5011));
    assert_eq!(final_ack.control, 0x31);
    assert!(frames(a.receive(final_ack, 5012)).is_empty());
    assert_eq!(a.sessions[0].as_ref().unwrap().buffered(), 0);
}
#[test]
fn lost_information_retransmits_after_poll() {
    let (mut a, mut b) = pair();
    let original = one(a.send(0, b"retry", 10).unwrap());
    let poll = one(a.tick(5010));
    let ack = one(b.receive(poll, 5011));
    let retry = one(a.receive(ack, 5012));
    assert_eq!(retry.info, original.info);
    assert_eq!(retry.control, original.control);
    let ack = one(b.receive(retry, 5013));
    a.receive(ack, 5014);
    assert_eq!(a.sessions[0].as_ref().unwrap().buffered(), 0);
}
#[test]
fn busy_peer_stops_queue_then_resumes() {
    let (mut a, _) = pair();
    a.receive(response("TEST-2", "TEST-1", 5), 10);
    assert!(frames(a.send(0, b"wait", 11).unwrap()).is_empty());
    assert_eq!(one(a.tick(5010)).control, 0x11);
    let sent = one(a.receive(response("TEST-2", "TEST-1", 0x11), 5011));
    assert_eq!(sent.info, b"wait");
}
#[test]
fn eight_incoming_sessions_and_ninth_refused() {
    let mut a = engine("LOCAL");
    for n in 1..=9 {
        let mut peer = engine(&format!("TEST-{n}"));
        let sabm = one(peer.connect(0, a.local.clone(), vec![], 0).unwrap());
        let answer = one(a.receive(sabm, 1));
        assert_eq!(answer.control, if n <= 8 { 0x73 } else { 0x1f });
    }
    assert_eq!(a.sessions.iter().flatten().count(), 8);
    assert_eq!(a.transport_lost().len(), 8);
    assert!(a.sessions.iter().all(Option::is_none));
}

#[test]
fn mailbox_limit_refuses_third_incoming_link_with_dm_from_mailbox_call() {
    let mut mailbox = engine("LOCAL-1");
    mailbox.incoming_limit = 2;
    for n in 1..=3 {
        let mut peer = engine(&format!("TEST-{n}"));
        let sabm = one(peer.connect(0, mailbox.local.clone(), vec![], 0).unwrap());
        let response = one(mailbox.receive(sabm, 1));
        assert_eq!(response.source, "LOCAL-1".parse().unwrap());
        assert_eq!(response.control, if n <= 2 { 0x73 } else { 0x1f });
    }
    assert_eq!(mailbox.sessions.iter().flatten().count(), 2);
}
#[test]
fn retry_limit_and_bounded_queue() {
    let (mut a, _) = pair();
    assert!(a.send(0, &vec![0; 16385], 10).is_err());
    a.send(0, &vec![0; 16384], 10).unwrap();
    assert!(a.send(0, &[1], 10).is_err());
    for n in 1..=11 {
        a.tick(10 + n * 5000);
    }
    assert!(a.sessions[0].is_none());
}
#[test]
fn invalid_ack_resets_without_silently_losing_state() {
    let (mut a, _) = pair();
    a.send(0, b"data", 10).unwrap();
    let events = a.receive(response("TEST-2", "TEST-1", 0xe1), 11);
    assert!(
        events
            .iter()
            .any(|e| matches!(e,Event::Status(_,s) if s.contains("Invalid N(R)")))
    );
    assert_eq!(one(events).control, 0x3f);
    assert_eq!(a.sessions[0].as_ref().unwrap().state, State::Connecting);
}
#[test]
fn digipeater_path_requires_h_bits_and_reverses() {
    let mut a = engine("TEST-1");
    let mut b = engine("TEST-2");
    let mut sabm = one(a
        .connect(
            0,
            b.local.clone(),
            vec!["DIGI-1".parse().unwrap(), "DIGI-2".parse().unwrap()],
            0,
        )
        .unwrap());
    assert!(b.receive(sabm.clone(), 1).is_empty());
    for (_, h) in &mut sabm.path {
        *h = true;
    }
    let ua = one(b.receive(sabm, 2));
    assert_eq!(ua.path[0].0.to_string(), "DIGI-2");
    assert!(ua.path.iter().all(|(_, h)| !*h));
}
#[test]
fn remote_window_and_reject_recovery() {
    let (mut a, _) = pair();
    let mut f = response("TEST-2", "TEST-1", 2);
    f.command = true;
    f.response = false;
    f.pid = Some(240);
    f.info = b"data".to_vec();
    assert_eq!(one(a.receive(f.clone(), 10)).control, 9); // Missing N(S)=0.
    assert!(a.receive(f.clone(), 11).is_empty()); // One REJ per exception.
    for seq in 0..7 {
        f.control = seq << 1;
        let events = a.receive(f.clone(), 20 + seq as u64);
        assert!(events.iter().any(|e| matches!(e, Event::Data(..))));
        assert_eq!(one(events).control, 1 | ((seq + 1) << 5));
    }
}

#[test]
fn srej_retransmits_only_the_requested_frame() {
    let (mut a, _) = pair();
    let original = one(a.send(0, b"pending", 10).unwrap());

    // N(R)=V(S) with P/F=0 requests a frame that has not been sent and must
    // not acknowledge the outstanding frame either.
    let events = a.receive(response("TEST-2", "TEST-1", 13 | (1 << 5)), 11);
    assert!(frames(events).is_empty());
    assert_eq!(a.sessions[0].as_ref().unwrap().buffered(), b"pending".len());

    // N(R)=N(S) requests the one outstanding frame, so it is retransmitted.
    let retry = one(a.receive(response("TEST-2", "TEST-1", 13), 12));
    assert_eq!(retry.control, original.control);
    assert_eq!(retry.info, original.info);
    assert_eq!(a.sessions[0].as_ref().unwrap().buffered(), b"pending".len());
}

#[test]
fn srej_poll_final_acknowledges_then_retransmits_requested_frame() {
    let (mut a, _) = pair();
    let original = one(a.send(0, b"pending", 10).unwrap());
    assert_eq!(one(a.tick(5010)).control, 0x11); // RR command, P=1.

    // A final SREJ for N(S)=0 acknowledges no earlier frames and selects
    // exactly the outstanding frame for retransmission.
    let final_srej = response("TEST-2", "TEST-1", 13 | 16);
    let retry = one(a.receive(final_srej, 11));
    assert_eq!(retry.control, original.control);
    assert_eq!(retry.info, original.info);
}

#[test]
fn extended_mode_is_accepted_and_disabled_listener_refuses_basic_connect() {
    let mut disabled = engine("DISABL");
    let mut peer = engine("TEST");
    let mut sabm = one(peer.connect(0, disabled.local.clone(), vec![], 0).unwrap());
    sabm.control = 0x7f;
    assert_eq!(one(disabled.receive(sabm, 1)).control, 0x1f);
    assert!(disabled.sessions.iter().all(Option::is_none));

    let mut a = engine("LOCAL");
    a.modulo_128 = true;
    let mut peer2 = engine("TEST2");
    let mut sabm = one(peer2.connect(0, a.local.clone(), vec![], 0).unwrap());
    sabm.control = 0x7f;
    assert_eq!(one(a.receive(sabm.clone(), 1)).control, 0x73);
    assert!(a.sessions[0].as_ref().unwrap().extended);
    let mut other = engine("OTHER");
    let mut basic = one(other.connect(0, a.local.clone(), vec![], 0).unwrap());
    basic.control = 0x3f;
    a.accept = false;
    assert_eq!(one(a.receive(basic, 2)).control, 0x1f);
}

#[test]
fn connect_selection_reuses_existing_without_rf_or_state_reset() {
    let (mut a, _) = pair();
    a.send(0, b"pending", 10).unwrap();
    let (id, events) = a
        .connect_selected(7, "test-2".parse().unwrap(), vec![], 11)
        .unwrap();
    assert_eq!(id, 0);
    assert!(
        events
            .iter()
            .all(|e| matches!(e,Event::Notice(0,s) if s.contains("Switched to session 1")))
    );
    assert_eq!(a.sessions[0].as_ref().unwrap().state, State::Connected);
    assert_eq!(a.sessions[0].as_ref().unwrap().buffered(), 7);
    assert_eq!(a.sessions.iter().flatten().count(), 1);
}

#[test]
fn disconnect_cancels_connecting_sequence_without_disc_or_sabm_retry() {
    let mut a = engine("TEST-1");
    let sabm = one(a.connect(0, "TEST-2".parse().unwrap(), vec![], 0).unwrap());
    assert_eq!(sabm.control, 0x3f);
    let events = a.disconnect(0, 1).unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Status(0, message) if message == "Connection cancelled"
    )));
    assert!(frames(events).is_empty());
    assert!(a.sessions[0].is_none());
    assert!(a.tick(10_000).is_empty());
}

#[test]
fn xid_after_sabm_ua_negotiates_extended_mode_before_data() {
    let mut a = engine("TEST-1");
    let mut b = engine("TEST-2");
    a.autonegotiation = true;
    a.modulo_128 = true;
    b.modulo_128 = true;

    let sabm = one(a.connect(0, b.local.clone(), vec![], 0).unwrap());
    assert_eq!(sabm.control, 0x7f);
    let ua = one(b.receive(sabm, 1));
    assert_eq!(ua.control, 0x73);
    let xid = one(a.receive(ua, 2));
    assert_eq!(xid.control, 0xbf);
    assert!(xid.summary().contains("XID"));
    assert_eq!(&xid.info[..4], &[0x82, 0x80, 0x00, 0x17]);
    assert_eq!(xid.info[11], 0xa8);
    assert_eq!(a.sessions[0].as_ref().unwrap().state, State::Negotiating);

    // A BBS can send its prompt immediately after UA, while the optional XID
    // exchange is still pending. The established modulo-8 link must carry it.
    let early_data = one(b.send(0, b"early prompt", 3).unwrap());
    let early_events = a.receive(early_data, 4);
    assert!(
        early_events
            .iter()
            .any(|event| matches!(event, Event::Data(0, data) if data == b"early prompt"))
    );
    let early_ack = one(early_events);
    b.receive(early_ack, 5);

    let mut xid_reply = one(b.receive(xid.clone(), 6));
    assert_eq!(xid_reply.control, 0xbf);
    assert!(!xid_reply.command);
    // Representative peer XID with SREJ/REJ: our selected function is REJ.
    xid_reply.info[10..13].copy_from_slice(&[0x86, 0xa8, 0x22]);
    let completion = a.receive(xid_reply, 7);
    assert!(completion.iter().any(|event| matches!(event, Event::Established(0))));
    assert!(a.sessions[0].as_ref().unwrap().extended);
    assert!(b.sessions[0].as_ref().unwrap().extended);

    for n in 0..130u16 {
        let data = one(a.send(0, b"x", 10 + u64::from(n)).unwrap());
        assert!(data.extended);
        assert_eq!((data.control >> 1) & 0x7f, n & 0x7f);
        let wire = data.encode();
        let decoded = Frame::decode_with_mode(&wire, true).unwrap();
        assert_eq!(decoded.control, data.control);
        let ack = one(b.receive(decoded, 1000 + u64::from(n)));
        assert!(ack.extended);
        a.receive(ack, 2000 + u64::from(n));
    }
}

#[test]
fn local_text_queues_while_xid_negotiation_completes() {
    let mut a = engine("TEST-1");
    let mut b = engine("TEST-2");
    a.autonegotiation = true;
    a.modulo_128 = true;
    b.modulo_128 = true;

    let sabm = one(a.connect(0, b.local.clone(), vec![], 0).unwrap());
    let ua = one(b.receive(sabm, 1));
    let xid = one(a.receive(ua, 2));

    assert_eq!(a.sessions[0].as_ref().unwrap().state, State::Negotiating);
    assert!(frames(a.send(0, b"operator text\r", 3).unwrap()).is_empty());
    assert_eq!(a.sessions[0].as_ref().unwrap().buffered(), 14);

    let xid_reply = one(b.receive(xid, 4));
    let text = one(a.receive(xid_reply, 5));
    assert!(text.extended);
    assert_eq!(text.info, b"operator text\r");

    let ack = one(b.receive(text, 8));
    a.receive(ack, 9);
    assert_eq!(a.sessions[0].as_ref().unwrap().buffered(), 0);
}

#[test]
fn incoming_data_does_not_delay_xid_fallback_or_queued_local_text() {
    let mut a = engine("TEST-1");
    let mut b = engine("TEST-2");
    a.autonegotiation = true;

    let sabm = one(a.connect(0, b.local.clone(), vec![], 0).unwrap());
    let ua = one(b.receive(sabm, 1));
    let xid = one(a.receive(ua, 2));
    let _ignored_xid_reply = b.receive(xid, 3);

    let prompt = one(b.send(0, b"station welcome\r", 4).unwrap());
    let received = a.receive(prompt, 5);
    assert!(
        received
            .iter()
            .any(|event| matches!(event, Event::Data(0, data) if data == b"station welcome\r"))
    );
    assert!(frames(a.send(0, b"/mh\r", 6).unwrap()).is_empty());
    let fallback = a.tick(5_003);
    assert!(
        fallback
            .iter()
            .any(|event| matches!(event, Event::Status(0, text) if text.contains("XID timeout")))
    );
    let sent = frames(fallback);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].info, b"/mh\r");
    assert!(!sent[0].extended);
}

#[test]
fn xid_applies_peer_n1_t1_and_retry_limit_to_the_link() {
    let mut a = engine("TEST-1");
    let mut b = engine("TEST-2");
    a.autonegotiation = true;
    a.modulo_128 = true;
    b.modulo_128 = true;
    a.n2 = 3;
    b.paclen = 200;
    b.t1 = 9000;
    b.n2 = 4;

    let sabm = one(a.connect(0, b.local.clone(), vec![], 0).unwrap());
    let ua = one(b.receive(sabm, 1));
    let xid = one(a.receive(ua, 2));
    let mut peer_reply = one(b.receive(xid, 3));
    // Peer advertises 64-byte receive N1, T1=12 seconds and N2=6.
    peer_reply.info[15..17].copy_from_slice(&512u16.to_be_bytes());
    peer_reply.info[22..24].copy_from_slice(&12_000u16.to_be_bytes());
    peer_reply.info[26] = 6;
    let completion = a.receive(peer_reply, 4);
    assert!(completion.iter().any(|event| matches!(event, Event::Established(0))));
    let first = one(a.send(0, &[b'x'; 130], 4).unwrap());
    assert_eq!(first.info.len(), 64);
    assert!(a.tick(12_003).is_empty());
    let retry_events = a.tick(12_004);
    assert_eq!(one(retry_events).control & 0x0100, 0x0100);
    let mut final_ack = response("TEST-2", "TEST-1", 0x301);
    final_ack.extended = true;
    let second = one(a.receive(final_ack, 12_005));
    assert_eq!(second.info.len(), 64);
}

#[test]
fn peer_xid_negotiation_does_not_switch_active_modulo_8_link() {
    let mut a = engine("TEST-1");
    let mut b = engine("TEST-2");
    b.paclen = 48;
    b.t1 = 7000;
    b.n2 = 3;
    a.autonegotiation = true;
    let sabm = one(a.connect(0, b.local.clone(), vec![], 0).unwrap());
    let ua = one(b.receive(sabm, 1));
    let xid = one(a.receive(ua, 2));
    let reply = one(b.receive(xid, 3));
    assert_eq!(reply.control, 0xbf);
    assert_eq!(&reply.info[15..17], &2048u16.to_be_bytes());
    assert_eq!(&reply.info[22..24], &7000u16.to_be_bytes());
    assert_eq!(reply.info[26], 10); // max(local N2=3, peer N2=10)

    assert!(a.receive(reply, 4).iter().any(|event| matches!(event, Event::Established(0))));
    let sent = one(b.send(0, &[b'x'; 130], 4).unwrap());
    assert_eq!(sent.info.len(), 48);
    assert!(!sent.extended);
}

#[test]
fn xid_retry_limit_uses_the_larger_selected_n2() {
    let mut a = engine("TEST-1");
    let mut b = engine("TEST-2");
    a.autonegotiation = true;
    a.t1 = 1000;
    a.n2 = 1;
    b.t1 = 5000;
    b.n2 = 2;
    let sabm = one(a.connect(0, b.local.clone(), vec![], 0).unwrap());
    let ua = one(b.receive(sabm, 1));
    let xid = one(a.receive(ua, 2));
    let reply = one(b.receive(xid, 3));
    assert_eq!(reply.info[26], 2);
    let completion = a.receive(reply, 4);
    assert!(completion.iter().any(|event| matches!(event, Event::Established(0))));
    assert!(frames(a.tick(5004)).is_empty());
    assert!(frames(a.tick(10004)).is_empty());
    assert!(a.sessions[0].as_ref().is_some_and(|session| session.state == State::Connected));
}

#[test]
fn malformed_sabme_is_refused_without_transmitting_frmr() {
    let mut b = engine("TEST-2");
    let sabme = Frame {
        source: "TEST-1".parse().unwrap(),
        destination: b.local.clone(),
        path: vec![],
        command: true,
        response: false,
        control: 0x6f,
        extended: true,
        pid: None,
        info: vec![1],
    };
    let refusal = one(b.receive(sabme, 1));
    assert_eq!(refusal.control, 0x0f);
    assert!(b.sessions.iter().all(Option::is_none));
}

#[test]
fn xid_rejection_or_timeout_keeps_the_active_format() {
    let mut a = engine("TEST-1");
    a.autonegotiation = true;
    let sabm = one(a.connect(0, "TEST-2".parse().unwrap(), vec![], 0).unwrap());
    assert_eq!(sabm.control, 0x3f);
    let xid = one(a.receive(response("TEST-2", "TEST-1", 0x73), 1));
    assert_eq!(xid.control, 0xbf);
    assert!(frames(a.send(0, b"queued during XID\r", 2).unwrap()).is_empty());
    let fallback = a.receive(response("TEST-2", "TEST-1", 0x87), 2);
    assert!(
        frames(fallback)
            .iter()
            .any(|frame| frame.info == b"queued during XID\r")
    );
    assert_eq!(a.sessions[0].as_ref().unwrap().state, State::Connected);
    assert!(!a.sessions[0].as_ref().unwrap().extended);

    let mut c = engine("TEST-3");
    c.autonegotiation = true;
    c.connect(0, "TEST-4".parse().unwrap(), vec![], 0).unwrap();
    c.receive(response("TEST-4", "TEST-3", 0x73), 1);
    let fallback = c.tick(c.t1 + 1);
    assert!(frames(fallback).is_empty());
    assert_eq!(c.sessions[0].as_ref().unwrap().state, State::Connected);
}
#[test]
fn connect_selection_uses_selected_free_slot_then_first_free() {
    let mut a = engine("TEST-1");
    let (id, _) = a
        .connect_selected(3, "PEER-1".parse().unwrap(), vec![], 0)
        .unwrap();
    assert_eq!(id, 3);
    let (id, _) = a
        .connect_selected(3, "PEER-2".parse().unwrap(), vec![], 0)
        .unwrap();
    assert_eq!(id, 0);
    a.disconnect(0, 1).unwrap();
    let (id, events) = a
        .connect_selected(3, "PEER-2".parse().unwrap(), vec![], 2)
        .unwrap();
    assert_eq!(id, 0);
    assert_eq!(frames(events).len(), 1);
    assert_eq!(a.sessions[0].as_ref().unwrap().state, State::Connecting);
    let (id, _) = a
        .connect_selected(0, "PEER-3".parse().unwrap(), vec![], 3)
        .unwrap();
    assert_eq!(id, 1);
    for n in 4..=8 {
        a.connect_selected(0, format!("PEER-{n}").parse().unwrap(), vec![], 4)
            .unwrap();
    }
    assert!(
        a.connect_selected(0, "PEER-9".parse().unwrap(), vec![], 5)
            .is_err()
    );
    assert_eq!(a.sessions.iter().flatten().count(), 8);
    let (id, events) = a
        .connect_selected(2, "PEER-1".parse().unwrap(), vec![], 6)
        .unwrap();
    assert_eq!(id, 3);
    assert!(frames(events).is_empty());
}
