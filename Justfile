# Named local counterparts to the CI validation tiers. `fast` is the default
# AI iteration path: one canonical engine feature build, with no extra test
# feature matrices. `quality` and `ci-all` remain explicit gates.

default: fast

fast:
    cargo fmt --all -- --check
    cargo check -p wyrd-for-games --no-default-features --features "std,signal-f32" --locked

test-fast:
    cargo test -p wyrd-for-games --lib --locked

test-engine test:
    cargo test -p wyrd-for-games --test "{{test}}" --locked

format:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test-workspace:
    cargo test --workspace --all-targets --locked

quality:
    just format
    just lint
    just test-workspace

test-signal features:
    cargo test -p wyrd-for-games --no-default-features --features "{{features}}"
    cargo test -p wyrd-for-games --no-default-features --features "{{features}},serde-ron"
    cargo test -p wyrd-for-games --no-default-features --features "{{features}},serde-json"
    cargo test -p wyrd-for-games --no-default-features --features "{{features}},serde-ron,serde-json"
    cargo test -p wyrd-for-games --no-default-features --features "{{features}},schema"

dual-signal:
    just test-signal "std,signal-f32"
    just test-signal "std,signal-i32"

test-bevy:
    cargo test -p wyrd-for-games-bevy
    cargo build -p wyrd-for-games-bevy --example and_door

test-docs:
    cargo test -p wyrd-for-games --doc --no-default-features --features "std,signal-f32,serde-ron,serde-json,schema" --locked

compile-workflows:
    gh aw compile

publish-core:
    cargo publish -p wyrd-for-games --locked

msrv-no-std:
    cargo +1.75 check -p wyrd-for-games --no-default-features --features "alloc,signal-f32"
    cargo +1.75 check -p wyrd-for-games --no-default-features --features "alloc,signal-i32"

coverage:
    cargo llvm-cov --workspace --exclude wyrd-for-games-bevy --ignore-filename-regex '(^|/)crates/wyrd-for-games/src/examples/' --lcov --output-path lcov-f32.info
    bash scripts/check-coverage-source-lines.sh crates/wyrd-for-games/src '(^|/)crates/wyrd-for-games/src/examples/'
    cargo llvm-cov clean --workspace
    cargo llvm-cov -p wyrd-for-games --no-default-features --features "std,signal-i32" --ignore-filename-regex '(^|/)crates/wyrd-for-games/src/examples/' --lcov --output-path lcov-i32.info
    bash scripts/check-coverage-source-lines.sh crates/wyrd-for-games/src '(^|/)crates/wyrd-for-games/src/examples/'
    cargo llvm-cov clean --workspace
    cargo llvm-cov -p wyrd-for-games --no-default-features --features "std,signal-f32,serde-ron" --ignore-filename-regex '(^|/)crates/wyrd-for-games/src/examples/' --summary-only
    bash scripts/check-coverage-source-lines.sh crates/wyrd-for-games/src '(^|/)crates/wyrd-for-games/src/examples/'
    cargo llvm-cov clean --workspace
    cargo llvm-cov -p wyrd-for-games --no-default-features --features "std,signal-f32,serde-json" --ignore-filename-regex '(^|/)crates/wyrd-for-games/src/examples/' --summary-only
    bash scripts/check-coverage-source-lines.sh crates/wyrd-for-games/src '(^|/)crates/wyrd-for-games/src/examples/'
    cargo llvm-cov clean --workspace
    cargo llvm-cov -p wyrd-for-games-bevy --lcov --output-path lcov-bevy.info
    bash scripts/check-coverage-source-lines.sh crates/wyrd-for-games-bevy/src

publish-readiness:
    cargo package -p wyrd-for-games --locked --list
    cargo package -p wyrd-for-games-bevy --locked --list
    just test-docs
    RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps

ci-all:
    just quality
    just dual-signal
    just test-bevy
    just msrv-no-std
    just coverage
    just publish-readiness
