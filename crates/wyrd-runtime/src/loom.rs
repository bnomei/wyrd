use wyrd_core::{
    is_truthy, CalcOp, CompareOp, FlagPriority, KnotId, KnotKind, PortSlot, Result, Signal,
    TimerMode, ONE, ZERO,
};
use wyrd_graph::Weave;

use crate::bind::Runtime;

impl Runtime {
    /// One settle pass. Never panics. No topology alloc after bind.
    pub fn loom(&mut self, _weave: &Weave) -> Result<()> {
        let n = self.knots.len();

        // 1. Zero input ports using precomputed slot lists.
        for ki in 0..n {
            let id = KnotId(ki as u16);
            // input_slots[ki] is stable; clear without holding borrow across set_port
            let slot_count = self.input_slots[ki].len();
            for si in 0..slot_count {
                let slot = self.input_slots[ki][si];
                self.set_port(id, slot, ZERO);
            }
        }

        // 2. Seed Sense outputs.
        for ki in 0..n {
            let id = KnotId(ki as u16);
            match &self.knots[ki].kind {
                KnotKind::Constant { value } => {
                    let v = *value;
                    self.set_port(id, PortSlot(0), v);
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

        // 3. Topo eval (topo is &self.topo — no clone).
        let topo_len = self.topo.len();
        for ti in 0..topo_len {
            let kid = self.topo[ti];
            self.gather_inputs(kid);
            self.eval_knot(kid);
        }

        Ok(())
    }

    fn gather_inputs(&mut self, kid: KnotId) {
        let ki = kid.0 as usize;
        // Stack buffer: max ports per knot is 8.
        let mut tmp: [(PortSlot, Signal); 8] = [(PortSlot(0), ZERO); 8];
        let n = self.inbound[ki].len().min(8);
        for i in 0..n {
            let (f, fs, ts) = self.inbound[ki][i];
            tmp[i] = (ts, self.get_port(f, fs));
        }
        for i in 0..n {
            self.set_port(kid, tmp[i].0, tmp[i].1);
        }
    }

    fn eval_knot(&mut self, kid: KnotId) {
        let ki = kid.0 as usize;
        // Copy discriminant data without cloning String-bearing variants.
        let tag = KindTag::from_kind(&self.knots[ki].kind);
        match tag {
            KindTag::Sense => {}
            KindTag::Not => {
                let i = self.get_port(kid, PortSlot(0));
                let o = if is_truthy(i) { ZERO } else { ONE };
                self.set_port(kid, PortSlot(1), o);
            }
            KindTag::And { arity } => {
                let mut ok = true;
                for s in 0..arity {
                    if !is_truthy(self.get_port(kid, PortSlot(s))) {
                        ok = false;
                        break;
                    }
                }
                self.set_port(kid, PortSlot(arity), if ok { ONE } else { ZERO });
            }
            KindTag::Or { arity } => {
                let mut ok = false;
                for s in 0..arity {
                    if is_truthy(self.get_port(kid, PortSlot(s))) {
                        ok = true;
                        break;
                    }
                }
                self.set_port(kid, PortSlot(arity), if ok { ONE } else { ZERO });
            }
            KindTag::RisingFromZero => {
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
            KindTag::Compare { op, rhs_const } => {
                let lhs = self.get_port(kid, PortSlot(0));
                let rhs = if let Some(c) = rhs_const {
                    crate::from_count(c)
                } else {
                    self.get_port(kid, PortSlot(1))
                };
                let o = if compare(op, lhs, rhs) { ONE } else { ZERO };
                self.set_port(kid, PortSlot(2), o);
            }
            KindTag::Flag {
                priority,
                enable_toggle,
            } => {
                let set_l = self.get_port(kid, PortSlot(0));
                let reset_l = self.get_port(kid, PortSlot(1));
                let toggle_l = self.get_port(kid, PortSlot(2));
                let set = is_truthy(set_l);
                let reset = is_truthy(reset_l);
                let toggle =
                    enable_toggle && !is_truthy(self.prev_in[ki]) && is_truthy(toggle_l);
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
            KindTag::Counter => {
                let inc = self.get_port(kid, PortSlot(0));
                let dec = self.get_port(kid, PortSlot(1));
                let reset = self.get_port(kid, PortSlot(2));
                if is_truthy(reset) {
                    self.counter[ki] = 0;
                }
                if !is_truthy(self.prev_in[ki]) && is_truthy(inc) {
                    self.counter[ki] = self.counter[ki].saturating_add(1);
                }
                if !is_truthy(self.prev_dec[ki]) && is_truthy(dec) {
                    self.counter[ki] = self.counter[ki].saturating_sub(1);
                }
                self.prev_in[ki] = inc;
                self.prev_dec[ki] = dec;
                self.set_port(kid, PortSlot(3), crate::from_count(self.counter[ki]));
            }
            KindTag::TimerPulseHold { ticks } => {
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
            KindTag::TimerFedCountdown { ticks } => {
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
            KindTag::Delay { ticks } => {
                let i = self.get_port(kid, PortSlot(0));
                if ticks == 0 {
                    self.set_port(kid, PortSlot(1), i);
                } else {
                    let len = self.delay_len[ki] as usize;
                    let off = self.delay_off[ki] as usize;
                    let head = self.delay_head[ki] as usize;
                    let o = self.delay_buf[off + head];
                    self.delay_buf[off + head] = i;
                    let next = head + 1;
                    self.delay_head[ki] = if next >= len { 0 } else { next as u16 };
                    self.set_port(kid, PortSlot(1), o);
                }
            }
            KindTag::Calc { op } => {
                let a = self.get_port(kid, PortSlot(0));
                let b = self.get_port(kid, PortSlot(1));
                let o = match op {
                    CalcOp::Add => wyrd_core::signal_ops::sat_add(a, b),
                    CalcOp::Sub => wyrd_core::signal_ops::sat_sub(a, b),
                    CalcOp::Mul => wyrd_core::signal_ops::mul(a, b),
                    CalcOp::Div => wyrd_core::signal_ops::div(a, b),
                };
                self.set_port(kid, PortSlot(2), o);
            }
            KindTag::Abs => {
                let i = self.get_port(kid, PortSlot(0));
                #[cfg(feature = "signal-f32")]
                let o = if i < 0.0 { -i } else { i };
                #[cfg(feature = "signal-i32")]
                let o = i.saturating_abs();
                self.set_port(kid, PortSlot(1), o);
            }
            KindTag::Neg => {
                let i = self.get_port(kid, PortSlot(0));
                #[cfg(feature = "signal-f32")]
                let o = -i;
                #[cfg(feature = "signal-i32")]
                let o = i.saturating_neg();
                self.set_port(kid, PortSlot(1), o);
            }
            KindTag::Map {
                in_min,
                in_max,
                out_min,
                out_max,
            } => {
                let i = self.get_port(kid, PortSlot(0));
                let o = map_linear(i, in_min, in_max, out_min, out_max);
                self.set_port(kid, PortSlot(1), o);
            }
            KindTag::SignalOut => {
                let v = self.get_port(kid, PortSlot(0));
                if let Some(path) = self.knots[ki].path {
                    self.push_signal_out(path, v);
                }
            }
            KindTag::EmitCommand => {
                let trig = self.get_port(kid, PortSlot(0));
                // Optional enable: unconnected → treated as ONE (enabled).
                let enable = if self.inbound[ki]
                    .iter()
                    .any(|&(_, _, ts)| ts == PortSlot(1))
                {
                    self.get_port(kid, PortSlot(1))
                } else {
                    ONE
                };
                let payload = self.get_port(kid, PortSlot(2));
                let prev = self.prev_in[ki];
                if !is_truthy(prev) && is_truthy(trig) && is_truthy(enable) {
                    if let Some(cmd) = self.knots[ki].cmd {
                        self.push_emit(cmd, payload);
                    }
                }
                self.prev_in[ki] = trig;
            }
        }
    }
}

/// Copyable dispatch tag — avoids cloning KnotKind Strings each loom.
#[derive(Clone, Copy)]
enum KindTag {
    Sense,
    Not,
    And { arity: u8 },
    Or { arity: u8 },
    RisingFromZero,
    Compare {
        op: CompareOp,
        rhs_const: Option<i32>,
    },
    Flag {
        priority: FlagPriority,
        enable_toggle: bool,
    },
    Counter,
    TimerPulseHold { ticks: u16 },
    TimerFedCountdown { ticks: u16 },
    Delay { ticks: u16 },
    Calc { op: CalcOp },
    Abs,
    Neg,
    Map {
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    },
    SignalOut,
    EmitCommand,
}

impl KindTag {
    fn from_kind(k: &KnotKind) -> Self {
        match k {
            KnotKind::Constant { .. } | KnotKind::SignalIn | KnotKind::OnStart => KindTag::Sense,
            KnotKind::Not => KindTag::Not,
            KnotKind::And { arity } => KindTag::And { arity: *arity },
            KnotKind::Or { arity } => KindTag::Or { arity: *arity },
            KnotKind::RisingFromZero => KindTag::RisingFromZero,
            KnotKind::Compare { op, rhs_const } => KindTag::Compare {
                op: *op,
                rhs_const: *rhs_const,
            },
            KnotKind::Flag {
                priority,
                enable_toggle,
            } => KindTag::Flag {
                priority: *priority,
                enable_toggle: *enable_toggle,
            },
            KnotKind::Counter => KindTag::Counter,
            KnotKind::Timer { mode, ticks } => match mode {
                TimerMode::PulseHold => KindTag::TimerPulseHold { ticks: *ticks },
                TimerMode::FedCountdown => KindTag::TimerFedCountdown { ticks: *ticks },
            },
            KnotKind::Delay { ticks } => KindTag::Delay { ticks: *ticks },
            KnotKind::Calc { op } => KindTag::Calc { op: *op },
            KnotKind::Abs => KindTag::Abs,
            KnotKind::Neg => KindTag::Neg,
            KnotKind::Map {
                in_min,
                in_max,
                out_min,
                out_max,
            } => KindTag::Map {
                in_min: *in_min,
                in_max: *in_max,
                out_min: *out_min,
                out_max: *out_max,
            },
            KnotKind::SignalOut { .. } => KindTag::SignalOut,
            KnotKind::EmitCommand { .. } => KindTag::EmitCommand,
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
