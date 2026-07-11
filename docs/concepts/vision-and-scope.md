# Vision and scope: from knots to puzzle worlds

Wyrd is a library for making game rules readable, validated, and portable. It is inspired by the
clear puzzle language of Zelda-style adventures and the compositional learning model of Nintendo
Game Builder Garage: connect small, understandable behaviours and make the result visible.

Those are design references, not product compatibility claims. Wyrd does not ship a visual editor,
Nintendo assets, a Game Builder Garage importer, or a Zelda implementation.

## The three useful scales

| Scale | What you author in Wyrd | Example host responsibility |
| --- | --- | --- |
| Knot | A typed transformation, state cell, or edge detector | Sample a button, overlap, inventory fact, or analog value. |
| Mechanism | A validated local rule circuit | Apply a door state, play a sound, animate a mover, or grant an item. |
| Chamber and world | One or more local machines with named boundaries | Decide active rooms, perform spatial queries, persist progression, load scenes, and run physics. |

At the first scale, a `SignalIn` becomes a `SignalOut`. At the second, `And`, `Flag`, `Timer`,
`Counter`, `Map`, `Threshold`, and edge knots form a puzzle shape. At the third, the host joins
those shapes through named senses, outputs, and commands—not hidden graph edges between worlds.

```text
crate on pad ─┐
player on pad ├─► And ─► RisingFromZero ─► Flag ─► "shrine.gate.open"
relic placed ─┘                                  │
                                                   └─► And + exit overlap ─► EmitCommand
                                                                            "world.request_transition"
```

The [Tier D chamber recipe](../../crates/wyrd-for-games/README.md) implements that
shape end to end. It also emits a continuous `"shrine.bridge.target"` level. The graph selects a
target; the host performs the actual movement, collision, and animation.

## What Wyrd owns—and what it deliberately does not

| Wyrd owns | Your host owns |
| --- | --- |
| Graph topology, port and domain validation, explicit stateful knots, and deterministic settle order | Entities, components, doors, transforms, physics, overlap checks, pathfinding, rendering, and audio playback |
| Runtime-local dense handles and the per-frame outbox | Mapping world facts into senses and interpreting output paths or command names |
| Reusable `Pattern` expansion before bind | Save data, inventories, room loading, scene transitions, and cross-room progression |
| A bounded, validated DAG | Any feedback that crosses a host tick or a room boundary |

This boundary gives game code an honest vocabulary. A pressure plate is not a Wyrd object: the
host decides whether a player or movable crate occupies it, then writes that fact as a sense. A
door is not a Wyrd object either: the host reads `"shrine.gate.open"` and applies its own movement
and collision rules. The same Weave can therefore run inside a Bevy game, a custom engine, or a
constrained runtime.

## Compose a room, then compose a world

Build each mechanism from observable facts and named effects.

1. Name host observations by meaning, such as `crate_on_sun_pad`, `player_at_exit`, or
   `bridge_lever`.
2. Combine them into an explicit rule. Chain `And` or `Or` knots for a condition with more inputs
   than one knot accepts; use `Flag`, `Counter`, `Timer`, or `Delay` when the rule needs memory.
3. Use `SignalOut` for a level the host should continuously apply and `EmitCommand` behind an edge
   knot for a one-shot request.
4. Keep the effect name semantic and host-owned: `"gate.open"`, `"bridge.target"`, and
   `"world.request_transition"` are contracts between the graph and game code, not built-in Wyrd
   objects.
5. Bind the complete authored graph during setup or room load, resolve handles once, and use those
   handles in the tick loop.

`Pattern` lets an author package a reusable mechanism—such as a pulse hold or a guarded latch—then
include it many times with namespaced internals and named exports. Use this for repetition inside a
room; keep the game-level orchestration in the host where it can be saved, inspected, and changed
without pretending it is graph topology.

## Multiple rooms and instances

An engine-neutral integration can bind one `Runtime` for the active room, rebind from the next
room's authored asset when loading, and pass saved facts back in as senses. If your engine keeps
several chambers simulated at once, it can keep several independent runtimes instead.

`wyrd-for-games-bevy` exposes this latter shape as `WyrdWorld`: it stores independently bound
`WyrdInstance`s and looms every active instance during `WyrdSet::Loom`. Remove or stash an inactive
instance if it should stop ticking. There are no cross-instance Threads and runtime handles reject
another runtime, by design. Persist a fact such as `shrine_gate_open` in the host, then sample it
into the next chamber as a normal `SignalIn`.

## State without hidden cycles

Weaves are directed acyclic graphs. A direct feedback loop is rejected before execution. That makes
the rule's order and cost inspectable; it does not prevent game state. Put local memory in an
explicit `Flag`, `Counter`, `Timer`, `Delay`, or edge knot. Put world-level memory in the host and
feed it back on the next tick or in the next room.

This is especially useful for puzzle design: a gate can remain open after the pads clear, a timed
bridge can expire, and an exit trigger can fire only when a readiness level rises—all without a
surprising cycle.

## Continue with a runnable shape

The [examples index](../examples/README.md) maps common puzzle intentions to the executable
cookbook. Start with the smallest matching mechanism, then use Tier D to see how multiple
mechanisms form a chamber. For the reason Wyrd binds once before the hot loop, read the
[performance model](performance-model.md).
