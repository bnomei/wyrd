use std::vec::Vec;

use wyrd_core::{
    is_truthy, ports_of, CalcOp, CompareOp, FlagPriority, HostTime, KnotId, KnotKind, PortDir,
    PortSlot, Result, Signal, TimerMode, ONE, ZERO,
};
use wyrd_graph::Weave;

use crate::bind::Runtime;

impl Runtime {
    /// One settle pass. Never panics. Zero alloc after bind (outbox may grow once reserved).
    pub fn loom(&mut self, _weave: &Weave) -> Result<()> {
        // Snapshot kinds so we can mutably write ports.
        let kinds: Vec<KnotKind> = self.knots.iter().map(|k| k.kind.clone()).collect();

        // 1. Zero input ports (keep outputs until written)
        for (ki, kind) in kinds.iter().enumerate() {
            for p in ports_of(kind) {
                if p.dir == PortDir::In {
                    self.set_port(KnotId(ki as u16), p.slot, ZERO);
                }
            }
        }

        // 2. Seed Sense: Constant / SignalIn / OnStart write `out`
        for (ki, kind) in kinds.iter().enumerate() {
            let id = KnotId(ki as u16);
            match kind {
                KnotKind::Constant { value } => {
                    self.set_port(id, PortSlot(0), *value);
                }
                KnotKind::SignalIn => {
                    let v = self.sense_values[ki];
                    self.set_port(id, PortSlot(0), v);
                }
                KnotKind::OnStart => {
                    let v = if !self.on_start_done[ki] {
                        self.on_start_done[ki] = true;
                        ONE
                    } else {
                        ZERO
                    };
                    self.set_port(id, PortSlot(0), v);
                }
                _ => {}
            }
        }

        // 3. Eval in topo order; before each knot, gather inbound Threads into its inputs
        let topo = self.topo.clone();
        for &kid in &topo {
            self.gather_inputs(kid);
            self.eval_knot(kid);
        }

        Ok(())
    }

    fn gather_inputs(&mut self, kid: KnotId) {
        // copy from upstream outputs into this knot's input slots
        let threads = self.threads.clone();
        for (f, fs, t, ts) in threads {
            if t == kid {
                let v = self.get_port(f, fs);
                self.set_port(t, ts, v);
            }
        }
    }

    fn eval_knot(&mut self, kid: KnotId) {
        let ki = kid.0 as usize;
        let kind = self.knots[ki].kind.clone();
        match kind {
            KnotKind::Constant { .. } | KnotKind::SignalIn | KnotKind::OnStart => {}
            KnotKind::Not => {
                let i = self.get_port(kid, PortSlot(0));
                let o = if is_truthy(i) { ZERO } else { ONE };
                self.set_port(kid, PortSlot(1), o);
            }
            KnotKind::And { arity } => {
                let mut ok = true;
                for s in 0..arity {
                    if !is_truthy(self.get_port(kid, PortSlot(s))) {
                        ok = false;
                        break;
                    }
                }
                self.set_port(kid, PortSlot(arity), if ok { ONE } else { ZERO });
            }
            KnotKind::Or { arity } => {
                let mut ok = false;
                for s in 0..arity {
                    if is_truthy(self.get_port(kid, PortSlot(s))) {
                        ok = true;
                        break;
                    }
                }
                self.set_port(kid, PortSlot(arity), if ok { ONE } else { ZERO });
            }
            KnotKind::RisingFromZero => {
                let i = self.get_port(kid, PortSlot(0));
                let prev = self.prev_in[ki];
                let o = if !is_truthy(prev) && is_truthy(i) {
                    ONE
                } else {
                    ZERO
                };
                self.prev_in[ki] = i;
                self.set_port(kid, PortSlot(1), o);
            }
            KnotKind::Compare { op, rhs_const } => {
                let lhs = self.get_port(kid, PortSlot(0));
                let rhs = if let Some(c) = rhs_const {
                    crate::from_count(c)
                } else {
                    self.get_port(kid, PortSlot(1))
                };
                let o = if compare(op, lhs, rhs) { ONE } else { ZERO };
                self.set_port(kid, PortSlot(2), o);
            }
            KnotKind::Flag {
                priority,
                enable_toggle,
            } => {
                let set_l = self.get_port(kid, PortSlot(0));
                let reset_l = self.get_port(kid, PortSlot(1));
                let toggle_l = self.get_port(kid, PortSlot(2));
                let set = is_truthy(set_l);
                let reset = is_truthy(reset_l);
                // Toggle is rising-edge so held toggle does not flip every loom.
                let toggle = enable_toggle
                    && !is_truthy(self.prev_in[ki])
                    && is_truthy(toggle_l);
                let mut st = self.flag[ki];
                match priority {
                    FlagPriority::ResetWins => {
                        if reset {
                            st = false;
                        } else if set {
                            st = true;
                        } else if toggle {
                            st = !st;
                        }
                    }
                    FlagPriority::SetWins => {
                        if set {
                            st = true;
                        } else if reset {
                            st = false;
                        } else if toggle {
                            st = !st;
                        }
                    }
                }
                self.prev_in[ki] = toggle_l;
                self.flag[ki] = st;
                self.set_port(kid, PortSlot(3), if st { ONE } else { ZERO });
            }
            KnotKind::Counter => {
                // Same-tick: reset level first, then rising-edge inc/dec (D-counter-edge).
                let inc = self.get_port(kid, PortSlot(0));
                let dec = self.get_port(kid, PortSlot(1));
                let reset = self.get_port(kid, PortSlot(2));
                if is_truthy(reset) {
                    self.counter[ki] = 0;
                } else {
                    if !is_truthy(self.prev_in[ki]) && is_truthy(inc) {
                        self.counter[ki] = self.counter[ki].saturating_add(1);
                    }
                    if !is_truthy(self.prev_dec[ki]) && is_truthy(dec) {
                        self.counter[ki] = self.counter[ki].saturating_sub(1);
                    }
                }
                self.prev_in[ki] = inc;
                self.prev_dec[ki] = dec;
                self.set_port(kid, PortSlot(3), crate::from_count(self.counter[ki]));
            }
            KnotKind::Timer { mode, ticks } => match mode {
                // Rising edge on `start` loads `ticks`. Each loom while remaining > 0:
                // active=ONE and remaining--. Held start does not re-arm.
                TimerMode::PulseHold => {
                    let start = self.get_port(kid, PortSlot(0));
                    let prev = self.prev_in[ki];
                    if !is_truthy(prev) && is_truthy(start) {
                        self.timer_left[ki] = ticks;
                    }
                    self.prev_in[ki] = start;
                    if self.timer_left[ki] > 0 {
                        self.set_port(kid, PortSlot(1), ONE);
                        self.timer_left[ki] -= 1;
                    } else {
                        self.set_port(kid, PortSlot(1), ZERO);
                    }
                }
                // While `feed` truthy: arm on rising feed, count down each tick;
                // active only when countdown finished and still fed. Drop feed → reset.
                TimerMode::FedCountdown => {
                    let feed = self.get_port(kid, PortSlot(0));
                    let prev = self.prev_in[ki];
                    if is_truthy(feed) {
                        if !is_truthy(prev) {
                            self.timer_left[ki] = ticks;
                        }
                        if self.timer_left[ki] > 0 {
                            self.timer_left[ki] -= 1;
                        }
                        let active = if self.timer_left[ki] == 0 {
                            ONE
                        } else {
                            ZERO
                        };
                        self.set_port(kid, PortSlot(1), active);
                    } else {
                        self.timer_left[ki] = 0;
                        self.set_port(kid, PortSlot(1), ZERO);
                    }
                    self.prev_in[ki] = feed;
                }
            },
            KnotKind::Delay { ticks } => {
                let i = self.get_port(kid, PortSlot(0));
                if ticks == 0 {
                    self.set_port(kid, PortSlot(1), i);
                } else {
                    let len = self.delay_len[ki] as usize;
                    let off = self.delay_off[ki] as usize;
                    let head = self.delay_head[ki] as usize;
                    // Output sample from `ticks` looms ago (ring slot at head).
                    let o = self.delay_buf[off + head];
                    self.delay_buf[off + head] = i;
                    let next = head + 1;
                    self.delay_head[ki] = if next >= len { 0 } else { next as u16 };
                    self.set_port(kid, PortSlot(1), o);
                }
            }
            KnotKind::Calc { op } => {
                let a = self.get_port(kid, PortSlot(0));
                let b = self.get_port(kid, PortSlot(1));
                let o = match op {
                    CalcOp::Add => wyrd_core::sat_add(a, b),
                    CalcOp::Sub => wyrd_core::sat_sub(a, b),
                    CalcOp::Mul => wyrd_core::mul(a, b),
                    CalcOp::Div => wyrd_core::div(a, b),
                };
                self.set_port(kid, PortSlot(2), o);
            }
            KnotKind::Abs => {
                let i = self.get_port(kid, PortSlot(0));
                #[cfg(feature = "signal-f32")]
                let o = if i < 0.0 { -i } else { i };
                #[cfg(feature = "signal-i32")]
                let o = i.saturating_abs();
                self.set_port(kid, PortSlot(1), o);
            }
            KnotKind::Neg => {
                let i = self.get_port(kid, PortSlot(0));
                #[cfg(feature = "signal-f32")]
                let o = -i;
                #[cfg(feature = "signal-i32")]
                let o = i.saturating_neg();
                self.set_port(kid, PortSlot(1), o);
            }
            KnotKind::Map {
                in_min,
                in_max,
                out_min,
                out_max,
            } => {
                let i = self.get_port(kid, PortSlot(0));
                let o = map_linear(i, in_min, in_max, out_min, out_max);
                self.set_port(kid, PortSlot(1), o);
            }
            KnotKind::SignalOut { .. } => {
                let v = self.get_port(kid, PortSlot(0));
                if let Some(path) = self.knots[ki].path {
                    self.push_signal_out(path, v);
                }
            }
            KnotKind::EmitCommand { .. } => {
                // Rising edge on `trigger` only (held level must not spam).
                let trig = self.get_port(kid, PortSlot(0));
                let payload = self.get_port(kid, PortSlot(2));
                let prev = self.prev_in[ki];
                if !is_truthy(prev) && is_truthy(trig) {
                    if let Some(cmd) = self.knots[ki].cmd {
                        self.push_emit(cmd, payload);
                    }
                }
                self.prev_in[ki] = trig;
            }
        }
    }
}

fn compare(op: CompareOp, lhs: Signal, rhs: Signal) -> bool {
    match op {
        CompareOp::Eq => lhs == rhs,
        CompareOp::Ne => lhs != rhs,
        CompareOp::Lt => lhs < rhs,
        CompareOp::Lte => lhs <= rhs,
        CompareOp::Gt => lhs > rhs,
        CompareOp::Gte => lhs >= rhs,
    }
}

fn map_linear(i: Signal, in_min: Signal, in_max: Signal, out_min: Signal, out_max: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        if (in_max - in_min).abs() < f32::EPSILON {
            return out_min;
        }
        let t = ((i - in_min) / (in_max - in_min)).clamp(0.0, 1.0);
        out_min + t * (out_max - out_min)
    }
    #[cfg(feature = "signal-i32")]
    {
        let den = (in_max as i64) - (in_min as i64);
        if den == 0 {
            return out_min;
        }
        let t = ((i as i64) - (in_min as i64)).clamp(0, den);
        let span = (out_max as i64) - (out_min as i64);
        (out_min as i64 + t * span / den) as i32
    }
}

// silence unused HostTime in this module
#[allow(dead_code)]
fn _tick(t: HostTime) -> u64 {
    t.tick
}
