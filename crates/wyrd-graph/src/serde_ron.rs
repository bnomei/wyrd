//! RON load/save for author Weaves (feature `serde-ron`).

use std::string::String;

use wyrd_core::{NumericPath, Result, WyrdError};

use crate::validate::{validate, Budget};
use crate::weave::Weave;

/// Deserialize Weave from RON text. Rejects numeric path mismatch vs compiled feature.
pub fn from_ron(text: &str) -> Result<Weave> {
    let weave: Weave = ron::from_str(text).map_err(|_| WyrdError::Parse)?;
    if weave.numeric != NumericPath::compiled() {
        return Err(WyrdError::NumericMismatch);
    }
    validate(&weave, &Budget::default())?;
    Ok(weave)
}

/// Serialize Weave to RON (pretty).
///
/// Returns `Ok` for any well-formed `Weave`. Serialization failure is treated
/// as a programmer error (types are always RON-representable).
pub fn to_ron(weave: &Weave) -> Result<String> {
    Ok(ron::ser::to_string_pretty(weave, ron::ser::PrettyConfig::default())
        .expect("Weave is RON-serializable"))
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
        let s = to_ron(&w).unwrap();
        let w2 = from_ron(&s).unwrap();
        assert_eq!(w.id, w2.id);
        assert_eq!(w.knots.len(), w2.knots.len());
        let _ = ONE;
    }

    #[test]
    fn wrong_numeric_rejected() {
        let (b, _) = Weave::builder("x")
            .knot("c", KnotKind::constant(ONE))
            .unwrap();
        let w = b.build().unwrap();
        let s = to_ron(&w).unwrap();
        // Flip numeric tag in serialized form (feature-specific tag).
        #[cfg(feature = "signal-f32")]
        let wrong = s.replace("f32", "i32q16");
        #[cfg(feature = "signal-i32")]
        let wrong = s.replace("i32q16", "f32");
        let err = from_ron(&wrong);
        assert!(
            matches!(err, Err(WyrdError::NumericMismatch)),
            "got {err:?}"
        );
    }

    #[test]
    fn parse_error_on_garbage() {
        assert_eq!(from_ron("not ron {{{"), Err(WyrdError::Parse));
    }

    #[test]
    fn validate_failure_after_parse() {
        // Valid RON / invalid weave (empty knots). Numeric tag must match compiled path.
        #[cfg(feature = "signal-f32")]
        let text = r#"(
            id: "e",
            knots: [],
            threads: [],
            numeric: f32,
        )"#;
        #[cfg(feature = "signal-i32")]
        let text = r#"(
            id: "e",
            knots: [],
            threads: [],
            numeric: i32q16,
        )"#;
        assert!(matches!(
            from_ron(text),
            Err(WyrdError::Empty) | Err(WyrdError::Parse)
        ));
    }
}
