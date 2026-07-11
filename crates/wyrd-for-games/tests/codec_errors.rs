//! Public codec diagnostics stay actionable across supported wire formats.

#[cfg(any(feature = "serde-json", feature = "serde-ron"))]
use std::error::Error as _;

#[cfg(any(feature = "serde-json", feature = "serde-ron"))]
use wyrd::{KnotDef, KnotKind, NumericPath, SignalDomain, Weave, WeaveDef, ONE};

#[cfg(any(feature = "serde-json", feature = "serde-ron"))]
fn valid_weave() -> Weave {
    Weave::try_from(WeaveDef {
        id: "codec".into(),
        numeric: NumericPath::compiled(),
        knots: vec![KnotDef {
            id: "one".into(),
            kind: KnotKind::constant(ONE, SignalDomain::Level),
        }],
        threads: vec![],
    })
    .expect("fixture must be a valid weave")
}

#[cfg(feature = "serde-json")]
#[test]
fn json_codec_round_trips_and_exposes_actionable_errors() {
    let weave = valid_weave();
    let encoded = wyrd::to_json(&weave).expect("valid weave must serialize");
    assert!(encoded.contains('\n'));
    assert_eq!(wyrd::from_json(&encoded).unwrap(), weave);

    let parse = wyrd::from_json("{").expect_err("truncated JSON must fail");
    assert!(parse
        .to_string()
        .starts_with("JSON parse error at line 1, column "));
    assert!(parse.source().is_some());
    assert!(matches!(
        parse,
        wyrd::JsonCodecError::Parse {
            line: 1,
            column: 1,
            ..
        }
    ));

    let mut invalid = weave.to_def();
    invalid.knots.clear();
    let encoded_invalid = serde_json::to_string(&invalid).unwrap();
    let validation = wyrd::from_json(&encoded_invalid).expect_err("empty weave must be rejected");
    assert_eq!(
        validation.to_string(),
        "invalid JSON weave: weave 'codec' has no knots"
    );
    assert!(validation.source().is_some());
    assert!(matches!(
        validation,
        wyrd::JsonCodecError::Validation(wyrd::ValidationError::EmptyWeave { .. })
    ));
}

#[cfg(feature = "serde-ron")]
#[test]
fn ron_codec_round_trips_and_exposes_actionable_errors() {
    let weave = valid_weave();
    let encoded = wyrd::to_ron(&weave).expect("valid weave must serialize");
    assert!(encoded.contains('\n'));
    assert_eq!(wyrd::from_ron(&encoded).unwrap(), weave);

    let parse = wyrd::from_ron("(bad:").expect_err("truncated RON must fail");
    assert!(parse
        .to_string()
        .starts_with("RON parse error at line 1, column "));
    assert!(parse.source().is_some());
    assert!(matches!(
        parse,
        wyrd::RonCodecError::Parse {
            line: 1,
            column,
            ..
        } if column > 0
    ));

    let mut invalid = weave.to_def();
    invalid.knots.clear();
    let encoded_invalid = ron::ser::to_string(&invalid).unwrap();
    let validation = wyrd::from_ron(&encoded_invalid).expect_err("empty weave must be rejected");
    assert_eq!(
        validation.to_string(),
        "invalid RON weave: weave 'codec' has no knots"
    );
    assert!(validation.source().is_some());
    assert!(matches!(
        validation,
        wyrd::RonCodecError::Validation(wyrd::ValidationError::EmptyWeave { .. })
    ));
}
