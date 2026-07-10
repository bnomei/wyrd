# Ideation: rustdoc examples ← GBG / dungeon literacy

**Status:** research / ideation only (no code)  
**Date:** 2026-07-10  
**Inputs:** patterns-library, api-preview review 05, patterns_cookbook, SoG GBG mini report, Shacknews nodon list, Zelda pro synthesis, community GBG tutorials (web search + Tavily)

---

## 1. Goal of this track

Ship **rustdoc-compatible, compilable examples** that teach Wyrd the way GBG lessons teach Nodons:

- Input → Middle → Output layering  
- Small “lesson machines,” not full games  
- Stable port names and edge/timer semantics  

Not a goal: Nodon UI, player VS, or re-deriving the Knot catalog.

---

## 2. What GBG actually teaches (distilled)

### 2.1 Official pedagogy structure

- **Interactive Lessons:** seven multi-step primary lessons (+ quizzes) — the forced onboarding path.  
- **Nodopedia:** per-Nodon reference (settings, ports, hints).  
- **Color layers:** Red Input / Green Middle / Blue Output / Objects.  
- **Budgets:** free programming caps (~512 Nodons, ~1024 connections) — same spirit as Wyrd hard budgets.

### 2.2 Green (middle) primitives that map cleanly to Wyrd

| GBG Nodon | Wyrd Knot / recipe | Notes for examples |
| --- | --- | --- |
| Constant | `Constant` | Always show as Sense |
| Button / Stick | host → `SignalIn` | Never “Button Knot” |
| On-Start | `OnStart` | First-tick pulse |
| AND | `And` arity 2+ | Ports `in_0`… not `a`/`b` |
| NOT | `Not` | Boolean invert only |
| Comparison | `Compare` | `lhs`/`rhs`, ops `Gte`… |
| Calculator | `Calc` | Teach sparingly (budget) |
| Map | `Map` | Cheap remap; community “save Nodons” tip |
| Digitize | `Digitize` | Quantize levels |
| Square-Root / Abs / ±Invert | `Sqrt` / `Abs` / `Neg` | Catalog already |
| Trigger-from-0 | `RisingFromZero` | Exact GBG name match |
| Flag (off prioritized) | `Flag` ResetWins | GBG: both ports → **off wins** |
| Counter | `Counter` | Edge story: Wyrd already rising on inc/dec |
| Timer | `Timer` PulseHold / FedCountdown | GBG one Timer ≈ two modes in Wyrd |
| Random | `Random` + Seed | Gate + **edge consumers** |
| Wormhole I/O | **Pattern exports** only | Not free silent remote links |

### 2.3 Community “lesson machines” (tutorial vocabulary)

From community tutorials (countdown timer, button cooldown, Map guides, Nintendo tips health-via-Counter):

| Machine | Typical Nodon chain | Wyrd recipe |
| --- | --- | --- |
| Countdown → action | Timer → Output | PulseHold or Delay → SignalOut / Emit |
| Button cooldown / anti-spam | Button → Timer (lockout) | Rising → PulseHold → AND(not active, press) |
| Press-N | Button → Counter → Comparison | Counter → Compare(Gte) → Rising |
| Health gauge | hits → Counter dec → display | Counter + Map/SignalOut (host UI) |
| Flag gate | Button → Flag on + AND | Flag + And |
| Map remap | Stick → Map → move | SignalIn → Map → SignalOut |
| Timed window | Button → Timer → door open while active | MonostablePulse |

### 2.4 Zelda product constraints on examples

From pro Zelda synthesis (keep in every latch/timer story):

- Prefer **room-local** state; host reset on room exit.  
- Toggles that persist need **explicit reset** or they softlock.  
- Timed switches need **cues** (host) + **restart** (re-hit switch).  
- Multi-switch AND/OR expands test space — keep arity small in docs.  
- No free-running clocks as toys.

---

## 3. Coverage matrix (Wyrd today)

| Pattern / lesson | patterns-library | cookbook CI | rustdoc | api-preview |
| --- | --- | --- | --- | --- |
| AndGate / two-plate door | #1, #9 | #2 | no | broken ports |
| OrGate | #2 | no | no | — |
| RisingPulse | #3 | used inside | no | — |
| MonostablePulse | #4 | #1 | no | empty Pattern body |
| TimedHold (FedCountdown) | #5 | **missing** | no | **missing** |
| SrUnlock | #6 | partial #3 | no | wrong toggle |
| ToggleBit | #7 | #3 | no | set≠toggle |
| CounterUnlock | #8 | #4 | no | double-edge risk |
| MultiSwitchLatch | #10 | **missing** | no | mentioned only |
| DelayedPulse | #12 | #5 | no | — |
| AxisDigital / Threshold | #14 | no | no | — |
| SequenceTwo | #15 sketch | no | no | — |
| Emit + edge | act docs | tests only | no | emit spam bug |
| Map / Digitize / Sqrt | catalog | no cookbook | no | — |
| OnStart | Sense | no recipe | no | — |
| Host tick_once | host docs | yes | no | preview |

**Conclusion:** five CI recipes cover the *minimum* GBG middle core, but miss **FedCountdown literacy**, **MultiSwitchLatch**, **cooldown**, **Map**, and **OnStart**. Rustdoc is almost empty. api-preview must not be the teaching source until rewritten.

---

## 4. Proposed lesson ladder (for future rustdoc + cookbook)

### Tier A — rustdoc (≤10 lines, always compile)

1. Constant → Not → SignalOut  
2. Two SignalIn → And(`in_0`,`in_1`) → SignalOut  
3. bind + set_sense + loom + outbox assert  
4. ScriptedHost + tick_once (2 frames)  
5. validate fails inverted Map / steps=0 Digitize  

### Tier B — first five (already CI; promote via shared `cookbook` fns)

1. Monostable Pattern include  
2. Two-plate And door request  
3. Flag toggle + **reset**  
4. Counter → Compare(Gte) unlock  
5. Delayed pulse  

### Tier C — GBG / Zelda literacy gap-fill

| ID | Name | Graph sketch | Softlock note |
| --- | --- | --- | --- |
| C1 | MultiSwitchLatch | And → Rising → Flag.set; reset → Flag.reset | always wire reset |
| C2 | TimedHold | SignalIn → FedCountdown.feed → active | leave plate resets |
| C3 | PressN then window | Counter≥N → Rising → PulseHold | edge Compare before timer |
| C4 | ButtonCooldown | edge → shot + PulseHold cooling (**no** active→gate cycle; DAG) | anti-hold-spam + cue |
| C5 | AxisDigital | Threshold or Compare + Rising | one axis per Signal |
| C6 | Map remap | SignalIn → Map → SignalOut | teach ranges |
| C7 | Digitize steps | SignalIn → Digitize → out | bins endpoints |
| C8 | OnStart pulse | OnStart → Emit or Flag.set once | first tick only |
| C9 | Emit once | ok → Rising → Emit.trigger | never level→Emit |
| C10 | Or any-of keys | Or(2) → out | alternate keys |

### Tier D — stretch (book / later, not first rustdoc)

- SequenceTwo (A then B)  
- Random gated lottery (with Rising on result)  
- Pattern monostable params (duration as frozen stamp)  
- Soft room-reset: reseed + host clears Flag/Counter  

### Explicit non-goals for examples

- Wormhole-as-hidden-bus  
- Free-running clock loops  
- Calculator spaghetti / CPU demos  
- Bevy Entity as identity  
- Door mesh / portal cells in core  

---

## 5. GBG → Wyrd naming for docs prose

Use **Wyrd terms** in code; allow **one GBG/Zelda gloss** in prose:

| Prose gloss | Code |
| --- | --- |
| “button held” | `SignalIn` level |
| “just pressed” | `RisingFromZero` |
| “both plates” | `And` |
| “stays open until reset” | `Flag` |
| “press three times” | `Counter` + `Compare` |
| “window after press” | `Timer` PulseHold |
| “stand still for N ticks” | `Timer` FedCountdown |
| “door open request” | `SignalOut("door.open")` + host |
| “play SFX once” | `EmitCommand` on rising |

---

## 6. Doctest engineering notes

1. Prefer shared `cookbook` functions called from `///` examples and integration tests.  
2. `# fn main() -> Result<()> { ... Ok(()) }` harness for `?`.  
3. Dual path: default f32 doctests; one note for i32 Q16.  
4. `cargo test --doc` should stay fast (≤15 snippets).  
5. Fix review-05 port names before any api-preview revival.  
6. Counter: teach **internal** rising edge — no extra Rising on `inc` unless labeled “level host → edge”.  

---

## 7. Further research (optional, narrow)

| Job | Why | Status |
| --- | --- | --- |
| GBG Interactive Lessons 1–7 step list | Official lesson order | Fandom extract blocked; need manual / alternate source |
| Nodopedia Timer vs Counter exact ports | Mode parity with PulseHold/FedCountdown | Partial via Shacknews |
| Community cooldown canonical graph | Tier C4 | Confirmed as common tutorial theme |
| Full GBG primitive re-catalog | Not needed | Catalog already locked |

Tavily note (2026-07-10): `include_answer advanced` fails validation on current CLI; use plain `--json`. Fandom extract often fails fetch; prefer Shacknews + Nintendo news + community tutorials.

---

## 8. Recommended build order (when leaving ideation)

1. **Inventory freeze:** this matrix as checklist.  
2. **`cookbook` module** with 5 existing recipes as real functions.  
3. **Tier A** on `Weave` / `Runtime` / `tick_once`.  
4. **C1 + C2** (MultiSwitchLatch + TimedHold) — highest GBG/Zelda gap.  
5. **C4 cooldown + C9 Emit-once** — stop classic spam bugs.  
6. **C6 Map** — GBG “cheap math” literacy.  
7. Only then mdBook / rewrite api-preview.

---

## 9. Sources (in-repo + web)

**In-repo**

- `docs/primitives/patterns-library.md`  
- `docs/api-preview/reviews/05-pedagogy-and-patterns.md`  
- `docs/research/provenance/trigger-effect-wiring/raw/tvly-mini-gbg-report.md`  
- `docs/research/provenance/trigger-effect-wiring/raw/extract-gbg-shacknews.md`  
- `docs/research/provenance/trigger-effect-wiring/raw/tvly-pro-zelda-report.md`  
- `crates/wyrd-runtime/tests/patterns_cookbook.rs`  

**Web (session)**

- Interactive Lessons / Guided Lesson Nodon (fandom titles)  
- Digital Trends: GBG tips — 7 interactive lessons  
- Community: countdown timer, button cooldown (3 Nodons), Map guide  
- Nintendo tips: Counter as health-style gauge  
- GBG Flag wiki: on/off, off prioritized when both  
