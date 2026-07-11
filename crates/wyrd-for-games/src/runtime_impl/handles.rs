//! Runtime-owned public handles around compact internal indices.

macro_rules! runtime_handle {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name {
            pub(crate) owner: usize,
            pub(crate) index: u16,
        }

        impl $name {
            pub(crate) const fn new(owner: usize, index: u16) -> Self {
                Self { owner, index }
            }

            /// Return the compact per-runtime index.
            pub const fn get(self) -> u16 {
                self.index
            }
        }
    };
}

runtime_handle!(
    SenseId,
    "A host-writable SignalIn handle owned by one Runtime."
);
runtime_handle!(
    HostPathId,
    "An interned SignalOut path handle owned by one Runtime."
);
runtime_handle!(
    CmdId,
    "An interned EmitCommand handle owned by one Runtime."
);
runtime_handle!(
    KnotHandle,
    "A knot handle owned by one Runtime, for checked tooling access."
);
