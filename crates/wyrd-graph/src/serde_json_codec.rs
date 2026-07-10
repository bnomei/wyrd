//! JSON load/save for author Weaves (feature `serde-json`).
//!
//! Same schema and gates as RON: parse → numeric path → validate.
//! Codec only; does not change `KnotKind` / `Weave` types.

use std::string::String;

use wyrd_core::{NumericPath, Result, WyrdError};

use crate::validate::{validate, Budget};
use crate::weave::Weave;

/// Deserialize Weave from JSON text. Rejects numeric path mismatch vs compiled feature.
pub fn from_json(text: &str) -> Result<Weave> {
    let weave: Weave = serde_json::from_str(text).map_err(|_| WyrdError::Parse)?;
    if weave.numeric != NumericPath::compiled() {
        return Err(WyrdError::NumericMismatch);
    }
    validate(&weave, &Budget::default())?;
    Ok(weave)
}

/// Serialize Weave to pretty JSON.
///
/// Returns `Ok` for any well-formed `Weave`. Serialization failure is treated
/// as a programmer error (types are always JSON-representable).
pub fn to_json(weave: &Weave) -> Result<String> {
    Ok(serde_json::to_string_pretty(weave).expect("Weave is JSON-serializable"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Weave;
    use wyrd_core::{KnotKind, ONE};

    #[test]
    fn round_trip_and_door() {
        let (b, pa) = Weave::builder("door")
            .knot("plate_a", KnotKind::signal_in())
            .unwrap();
        let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
        let (b, _) = b.and2("both", pa, pb).unwrap();
        let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
        let w = b.wire_named("both", "out", "door", "in").build().unwrap();
        let s = to_json(&w).unwrap();
        assert!(s.contains("plate_a") || s.contains("SignalIn"));
        let w2 = from_json(&s).unwrap();
        assert_eq!(w.id, w2.id);
        assert_eq!(w.knots.len(), w2.knots.len());
        assert_eq!(w.threads.len(), w2.threads.len());
        let _ = ONE;
    }

    #[test]
    fn wrong_numeric_rejected() {
        let (b, _) = Weave::builder("x")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let w = b.build().unwrap();
        let s = to_json(&w).unwrap();
        #[cfg(feature = "signal-f32")]
        let wrong = s.replace("\"f32\"", "\"i32q16\"");
        #[cfg(feature = "signal-i32")]
        let wrong = s.replace("\"i32q16\"", "\"f32\"");
        let err = from_json(&wrong);
        assert!(
            matches!(err, Err(WyrdError::NumericMismatch)),
            "got {err:?}"
        );
    }

    #[test]
    fn parse_error_on_garbage() {
        assert_eq!(from_json("not json {{{"), Err(WyrdError::Parse));
    }

    #[test]
    fn validate_failure_after_parse() {
        #[cfg(feature = "signal-f32")]
        let text = r#"{
            "id": "e",
            "knots": [],
            "threads": [],
            "numeric": "f32"
        }"#;
        #[cfg(feature = "signal-i32")]
        let text = r#"{
            "id": "e",
            "knots": [],
            "threads": [],
            "numeric": "i32q16"
        }"#;
        assert!(matches!(
            from_json(text),
            Err(WyrdError::Empty) | Err(WyrdError::Parse)
        ));
    }

    /// When both codecs are enabled: Weave identity through RON ↔ JSON.
    #[cfg(feature = "serde-ron")]
    #[test]
    fn ron_json_round_trip_via_weave() {
        use crate::serde_ron::{from_ron, to_ron};

        let (b, pa) = Weave::builder("door")
            .knot("plate_a", KnotKind::signal_in())
            .unwrap();
        let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
        let (b, _) = b.and2("both", pa, pb).unwrap();
        let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
        let w = b.wire_named("both", "out", "door", "in").build().unwrap();

        let json = to_json(&w).unwrap();
        let w2 = from_json(&json).unwrap();
        let ron = to_ron(&w2).unwrap();
        let w3 = from_ron(&ron).unwrap();
        assert_eq!(w.id, w3.id);
        assert_eq!(w.knots.len(), w3.knots.len());
        assert_eq!(w.threads.len(), w3.threads.len());
        assert_eq!(w.numeric, w3.numeric);
    }
}
