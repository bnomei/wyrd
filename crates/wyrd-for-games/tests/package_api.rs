//! Downstream-style smoke test for the two public import shapes.

use wyrd::{weave, BindOpts, KnotKind, Runtime, SignalDomain};

#[test]
fn root_api_and_layer_namespaces_are_available() {
    let weave = weave! {
        id: "package-api";
        knots {
            source = KnotKind::signal_in(SignalDomain::Bool);
            sink = KnotKind::signal_out("package.api", SignalDomain::Bool);
        }
        threads {
            source.out -> sink.in;
        }
    }
    .expect("valid downstream weave");

    let runtime = Runtime::bind(weave, BindOpts::default()).expect("valid downstream runtime");
    assert!(runtime.path_id("package.api").is_some());

    let _: Option<wyrd::graph::KnotHandle> = None;
    let _: Option<wyrd::runtime::KnotHandle> = None;
    let _: wyrd::core::Signal = wyrd::core::ZERO;
}
