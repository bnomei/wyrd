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
pub fn to_ron(weave: &Weave) -> Result<String> {
    ron::ser::to_string_pretty(weave, ron::ser::PrettyConfig::default())
        .map_err(|_| WyrdError::Serialize)
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
        let mut w = b.build().unwrap();
        let s = to_ron(&w).unwrap();
        // Flip numeric tag in serialized form
        let wrong = if s.contains("f32") {
            s.replace("f32", "i32q16")
        } else {
            s.replace("i32q16", "f32")
        };
        assert!(
            matches!(from_ron(&wrong), Err(WyrdError::NumericMismatch)),
            "got {:?}",
            from_ron(&wrong)
        );
    }
}
