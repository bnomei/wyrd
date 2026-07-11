# Choose a puzzle shape

Every recipe below is executable Rust: its source appears in rustdoc and the tutorial ladder runs
it as an integration test. Use the smallest matching shape first, then combine shapes into a room
only when the rule needs it.

## Start from the behaviour

| You want to make… | Start with | Building blocks |
| --- | --- | --- |
| A door that opens while two things are present | Tier B, two-plate door | `SignalIn` → `And` → `SignalOut` |
| A door that stays open once a condition is solved | Tier C, multi-switch latch | `And` → `RisingFromZero` → `Flag` |
| A condition that must be sustained | Tier C, timed hold | `TimerMode::FedCountdown` |
| A reward after N distinct presses | Tier C, press-N window | `Counter` → `Compare` → edge → `Timer` |
| A button, area, or dialogue trigger that fires once | Tier C, emit once | `RisingFromZero` → `EmitCommand` |
| A continuous target for a host-owned mover | Tier C, map remap | `SignalIn` → `Map` → `SignalOut` |
| Several objects, a persistent gate, mover target, and an exit transition | Tier D, shrine chamber | Combined local mechanisms with a host-owned room handoff |

The catalog deliberately describes rules, not scene objects. A "two-plate door" means the host
writes two occupancy facts and applies a named output. One plate may be a player, a crate, a
companion, or any other world fact.

## Capstone: shrine chamber

Tier D combines the pieces without claiming that Wyrd owns a scene:

```text
crate on pad + player on pad + relic placed
                    │
                    ▼
            edge → persistent gate-open flag ──► host opens gate
                    │
bridge lever ───────┴───────────────────────────► host moves bridge to target
                    │
gate-open + player-at-exit → edge → request transition ─► host saves / loads room
```

Run it directly:

```bash
cargo test -p wyrd-for-games --test tutorial_ladder d01_shrine_chamber
cargo test -p wyrd-for-games --doc
```

Read the full source in the [`wyrd-for-games` cookbook](../../crates/wyrd-for-games/README.md). The test
proves three important properties: the gate remains open after the initial arrangement changes,
the bridge target is a continuous host contract, and the exit command fires only on entry rather
than every frame inside the trigger.

## Compose at the correct boundary

Use `Pattern` to package mechanisms that recur inside one Weave. Use independent `Runtime`s or
Bevy `WyrdInstance`s for concurrently active chambers. Use host save state and normal senses to
carry progress between rooms; instances do not share graph Threads. The
[vision and scope guide](../concepts/vision-and-scope.md) explains why this keeps both game logic
and world ownership legible.
