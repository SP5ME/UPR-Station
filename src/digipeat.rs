//! AX.25 layer-2 digipeater path handling.
use crate::ax25::{Call, Frame};

/// Mark this station's address as repeated when it is the next unrepeated hop.
/// AX.25 requires all earlier repeater H bits to be set before this station repeats.
pub fn mark_for_repeat(frame: &mut Frame, local: &Call) -> bool {
    if frame.source == *local || frame.destination == *local {
        return false;
    }

    for (call, repeated) in &mut frame.path {
        if *repeated {
            continue;
        }
        if call == local {
            *repeated = true;
            return true;
        }
        // AX.25 path order: a digi may not pass an earlier, still-unrepeated hop.
        return false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(source: &str, path: &[(&str, bool)]) -> Frame {
        Frame {
            destination: "DEST".parse().unwrap(),
            source: source.parse().unwrap(),
            path: path
                .iter()
                .map(|(call, repeated)| (call.parse().unwrap(), *repeated))
                .collect(),
            command: true,
            response: false,
            control: 3,
            extended: false,
            pid: Some(0xf0),
            info: b"payload".to_vec(),
        }
    }

    #[test]
    fn repeats_own_next_hop_and_marks_h_bit() {
        let local = "DIGI".parse().unwrap();
        let mut frame = frame("SRC", &[("WIDE1", true), ("DIGI", false), ("WIDE2", false)]);

        assert!(mark_for_repeat(&mut frame, &local));
        assert!(frame.path[0].1);
        assert!(frame.path[1].1);
        assert!(!frame.path[2].1);
        assert_eq!(Frame::decode(&frame.encode()).unwrap().path, frame.path);
    }

    #[test]
    fn does_not_repeat_when_an_earlier_hop_is_pending() {
        let local = "DIGI".parse().unwrap();
        let mut frame = frame("SRC", &[("WIDE1", false), ("DIGI", false)]);
        assert!(!mark_for_repeat(&mut frame, &local));
        assert!(frame.path.iter().all(|(_, repeated)| !repeated));
    }

    #[test]
    fn does_not_repeat_already_handled_or_endpoint_frames() {
        let local: Call = "DIGI".parse().unwrap();
        let mut repeated = frame("SRC", &[("DIGI", true)]);
        assert!(!mark_for_repeat(&mut repeated, &local));

        let mut endpoint = frame("DIGI", &[("DIGI", false)]);
        assert!(!mark_for_repeat(&mut endpoint, &local));
        assert!(!endpoint.path[0].1);
    }
}
