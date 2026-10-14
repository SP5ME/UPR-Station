use std::path::Path;
use ultimatepr::{
    automation::{Automation, validate},
    ax25::Frame,
    config::Config,
    link::{Engine, Event, State},
    mheard::Heard,
};
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
fn setup() -> (Engine, Engine, Automation) {
    setup_with_paclen(128)
}
fn setup_with_paclen(paclen: usize) -> (Engine, Engine, Automation) {
    let config = Config::parse("").unwrap();
    let mut service =
        Automation::new(&config, Path::new("target/automation-test/config.conf"), 0).unwrap();
    service.templates.text = [
        "Hello {peercall} from {mycall}".into(),
        "73 {peercall}".into(),
        "Info {mycall}".into(),
        String::new(),
    ];
    let mut local = Engine::new("LOCAL".parse().unwrap());
    let mut peer = Engine::new("PEER".parse().unwrap());
    local.paclen = paclen;
    peer.paclen = paclen;
    let sabm = frames(peer.connect(0, local.local.clone(), vec![], 0).unwrap()).remove(0);
    let established = local.receive(sabm, 1);
    let mut output = frames(service.process(&mut local, &Heard::default(), established, 1));
    assert_eq!(output.len(), 2);
    assert_eq!(output[0].control & !16, 0x63);
    peer.receive(output.remove(0), 2);
    let welcome = output.remove(0);
    assert_eq!(welcome.info, b"Hello PEER from LOCAL\r");
    let ack = frames(peer.receive(welcome, 3)).remove(0);
    local.receive(ack, 4);
    (local, peer, service)
}
#[test]
fn defaults_and_empty_beacon_offline_schedule() {
    let config = Config::parse("").unwrap();
    assert!(
        config.welcome
            && config.goodbye
            && config.remote_commands
            && config.beacon
            && config.digipeat
    );
    assert_eq!(config.beacon_seconds, 600);
    assert!(!Config::parse("digipeat_enabled=false").unwrap().digipeat);
    let mut service =
        Automation::new(&config, Path::new("target/automation-test/config.conf"), 0).unwrap();
    service.templates.text[3].clear();
    let mut engine = Engine::new("LOCAL".parse().unwrap());
    assert!(service.tick(&mut engine, 599999, true, false).is_empty());
    let beacon = frames(service.tick(&mut engine, 600000, true, false)).remove(0);
    assert_eq!(beacon.control, 3);
    assert_eq!(beacon.pid, Some(240));
    assert!(beacon.info.is_empty());
    assert_eq!(beacon.destination.to_string(), "BEACON");
    assert!(service.tick(&mut engine, 1200000, false, false).is_empty());
    assert!(service.tick(&mut engine, 1200001, true, false).is_empty());
    assert_eq!(
        frames(service.tick(&mut engine, 1800000, true, false)).len(),
        1
    );
}
#[test]
fn beacon_paths_are_per_interface() {
    let config = Config::parse(
        "callsign=LOCAL\n\
         interface_VHF=127.0.0.1:8001,0,0,true,\n\
         interface_NET=127.0.0.1:8002,0,1,true,\n\
         beacon_interface_VHF=true,DIGI1 DIGI2\n\
         beacon_interface_NET=true,DIGI3\n",
    )
    .unwrap();
    assert!(config.interfaces[0].beacon_enabled);
    assert_eq!(
        config.interfaces[0]
            .beacon_path
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["DIGI1", "DIGI2"]
    );
    assert!(config.interfaces[1].beacon_enabled);
    assert_eq!(config.interfaces[1].beacon_path[0].to_string(), "DIGI3");

    let service =
        Automation::new(&config, Path::new("target/automation-test/config.conf"), 0).unwrap();
    let engine = Engine::new("LOCAL".parse().unwrap());
    let ports = vec![
        ("VHF".into(), String::new(), true),
        ("NET".into(), String::new(), true),
    ];
    let events = service.beacon_events(&engine, &ports).unwrap();
    assert_eq!(events.len(), 2);
    let Event::TransmitOn(0, frame) = &events[0] else {
        panic!("expected beacon on VHF");
    };
    assert_eq!(
        frame
            .path
            .iter()
            .map(|(call, _)| call.to_string())
            .collect::<Vec<_>>(),
        ["DIGI1", "DIGI2"]
    );
    let Event::TransmitOn(1, frame) = &events[1] else {
        panic!("expected beacon on NET");
    };
    assert_eq!(frame.path[0].0.to_string(), "DIGI3");

    let mailbox = Engine::new("LOCAL-1".parse().unwrap());
    let mailbox_events = service.beacon_events(&mailbox, &ports).unwrap();
    assert_eq!(mailbox_events.len(), events.len());
    for (index, (main, mailbox)) in events.iter().zip(&mailbox_events).enumerate() {
        let (
            Event::TransmitOn(main_port, main_frame),
            Event::TransmitOn(mailbox_port, mailbox_frame),
        ) = (main, mailbox)
        else {
            panic!("expected interface-routed beacons");
        };
        assert_eq!(*main_port, index);
        assert_eq!(mailbox_port, main_port);
        assert_eq!(mailbox_frame.source.to_string(), "LOCAL-1");
        assert_eq!(mailbox_frame.destination, main_frame.destination);
        assert_eq!(mailbox_frame.path, main_frame.path);
    }
}

#[test]
fn new_interface_defaults_to_beacon_enabled_without_digipeaters() {
    let config = Config::parse(
        "interface_VHF=127.0.0.1:8001,0,0,true,\ninterface_NET=127.0.0.1:8002,0,1,true,",
    )
    .unwrap();
    assert!(
        config
            .interfaces
            .iter()
            .all(|interface| { interface.beacon_enabled && interface.beacon_path.is_empty() })
    );
}

#[test]
fn legacy_beacon_path_is_copied_to_existing_interfaces() {
    let config = Config::parse(
        "beacon_path=OLD1 OLD2\n\
         interface_VHF=127.0.0.1:8001,0,0,true,\n\
         interface_NET=127.0.0.1:8002,0,1,true,\n",
    )
    .unwrap();
    assert!(config.interfaces.iter().all(|interface| {
        interface
            .beacon_path
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            == ["OLD1", "OLD2"]
    }));
}
#[test]
fn incoming_commands_fragmented_acknowledged_and_duplicate_safe() {
    let (mut local, mut peer, mut service) = setup();
    let heard = Heard::default();
    let part = frames(peer.send(0, b"/in", 10).unwrap()).remove(0);
    let events = local.receive(part, 11);
    let ack = frames(service.process(&mut local, &heard, events, 11));
    assert_eq!(ack.len(), 1);
    peer.receive(ack[0].clone(), 12);
    let part = frames(peer.send(0, b"fo\r\n", 20).unwrap()).remove(0);
    let events = local.receive(part.clone(), 21);
    let output = frames(service.process(&mut local, &heard, events, 21));
    assert_eq!(output.len(), 2);
    assert_eq!(output[1].info, b"Info LOCAL\r");
    let events = local.receive(part, 22);
    assert_eq!(
        frames(service.process(&mut local, &heard, events, 22)).len(),
        1
    );
    assert!(
        service
            .process(
                &mut local,
                &heard,
                vec![Event::Data(0, b"/quit\r/port\r".to_vec())],
                3000
            )
            .iter()
            .all(|e| !matches!(e, Event::Transmit(_)))
    );
    let mh = service.process(
        &mut local,
        &heard,
        vec![Event::Data(0, b"/mh\r".to_vec())],
        4000,
    );
    assert_eq!(mh.len(), 1); // reply queued behind unacknowledged info
    assert!(local.sessions[0].as_ref().unwrap().buffered() > 11);
}
#[test]
fn goodbye_waits_for_ack_or_deadline() {
    let (mut local, mut peer, mut service) = setup();
    let bye = frames(service.disconnect(&mut local, 0, 10).unwrap()).remove(0);
    assert_eq!(bye.info, b"73 PEER\r");
    assert!(service.tick(&mut local, 11, true, false).is_empty());
    assert_eq!(local.sessions[0].as_ref().unwrap().state, State::Connected);
    let ack = frames(peer.receive(bye, 12)).remove(0);
    local.receive(ack, 13);
    assert_eq!(
        frames(service.tick(&mut local, 14, true, false))[0].control,
        0x53
    );
    let (mut local, _, mut service) = setup();
    service.disconnect(&mut local, 0, 10).unwrap();
    assert_eq!(
        frames(service.tick(&mut local, 10010, true, false))[0].control,
        0x53
    );
}
#[test]
fn outgoing_has_no_welcome_or_goodbye() {
    let (mut local, mut peer, mut service) = setup();
    let events = vec![Event::Established(0)];
    let output = service.process(&mut peer, &Heard::default(), events, 3000);
    assert!(frames(output).is_empty());
    assert_eq!(
        frames(service.disconnect(&mut peer, 0, 3001).unwrap())[0].control,
        0x53
    );
    assert!(service.disconnect(&mut local, 99, 0).is_err());
}
#[test]
fn templates_reject_unknown_variables_and_controls() {
    assert!(validate("{mycall} {myname} {mylocator} {peercall}", 0).is_ok());
    assert!(validate("{peercall}", 3).is_err());
    assert!(validate("{bad}", 0).is_err());
    assert!(validate("{mycall", 0).is_err());
    assert!(validate("\x1b[2J", 0).is_err());
    assert!(validate(&"x".repeat(4097), 0).is_err());
}

#[test]
fn commands_are_rx_only_in_both_session_directions() {
    for incoming in [true, false] {
        let (local, peer, mut service) = setup_with_paclen(256);
        let (mut terminal, mut remote) = if incoming {
            (local, peer)
        } else {
            (peer, local)
        };
        // Keep this command-routing test focused on command behavior. The
        // production PACLEN is smaller than the complete /help response.
        let heard = Heard::default();
        for (n, command) in ["/help", "/h", "/info", "/i", "/mh"].iter().enumerate() {
            let now = 3000 + n as u64 * 3000;
            // Sending text containing a command must only transmit it, never answer it.
            let data = format!("{command}\r");
            let tx = terminal.send(0, data.as_bytes(), now).unwrap();
            let output = frames(service.process(&mut terminal, &heard, tx, now));
            assert_eq!(output.len(), 1);
            assert_eq!(output[0].info, data.as_bytes());
            let ack = frames(remote.receive(output[0].clone(), now + 1)).remove(0);
            let events = terminal.receive(ack, now + 2);
            assert!(frames(service.process(&mut terminal, &heard, events, now + 2)).is_empty());
            // The same command received over AX.25 produces one answer after the ACK.
            let packet = frames(remote.send(0, data.as_bytes(), now + 3).unwrap()).remove(0);
            let events = terminal.receive(packet, now + 4);
            let output = frames(service.process(&mut terminal, &heard, events, now + 4));
            assert_eq!(output.len(), 2);
            remote.receive(output[0].clone(), now + 5);
            let expected = match *command {
                "/help" | "/h" => {
                    "Commands: /help /h - help; /info /i - station info; /mh [PORT|NUMBER] - latest 10 heard stations; /ports (/port) - list interfaces\r".to_owned()
                }
                "/info" | "/i" => format!("Info {}\r", terminal.local),
                _ => format!("MHeard {} (latest 10)\rNo stations heard\r", terminal.local),
            };
            assert_eq!(output[1].info, expected.as_bytes());
            let ack = frames(remote.receive(output[1].clone(), now + 6)).remove(0);
            terminal.receive(ack, now + 7);
        }
    }
}
#[test]
fn disabled_remote_commands_ignore_help_in_both_directions() {
    for incoming in [true, false] {
        let (local, peer, _) = setup();
        let mut terminal = if incoming { local } else { peer };
        let config = Config::parse("remote_commands=false").unwrap();
        let mut service =
            Automation::new(&config, Path::new("target/automation-test/config.conf"), 0).unwrap();
        let events = vec![Event::Data(0, b"/help\r/h\r/info\r/mh\r".to_vec())];
        assert!(frames(service.process(&mut terminal, &Heard::default(), events, 3000)).is_empty());
    }
}
