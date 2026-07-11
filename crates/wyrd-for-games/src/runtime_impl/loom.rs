//! Single-pass DAG settle: clear unwired ins, seed senses, topo eval, fill outbox.
//!
//! After a successful bind, loom is infallible and allocates no graph topology.
//! Dispatch uses bind-time [`KindTag`]s; inbound edges use CSR tables. Stateful
//! runes (Flag, Counter, Timer, Delay, edges) update per-knot storage here.

use crate::foundation::{
    is_truthy, CompareOp, FlagPriority, KnotId, PortSlot, Signal, SignalDomain, ONE, ZERO,
};

use crate::runtime_impl::bind::{Runtime, SenseSeed};
use crate::runtime_impl::kind_tag::KindTag;

#[allow(non_snake_case)]
const fn PortSlot(value: u8) -> PortSlot {
    PortSlot::new(value)
}

impl Runtime {
    /// One settle pass. Never panics. No topology alloc after bind.
    ///
    /// Order: zero unwired inputs → seed Constant / SignalIn / OnStart →
    /// topological eval of non-sense knots → acts append to the outbox.
    pub fn loom(&mut self) {
        for &idx in &self.clear_port_idx {
            debug_assert!(idx < self.port_vals.len());
            self.port_vals[idx] = ZERO;
        }

        let seed_n = self.sense_seeds.len();
        for si in 0..seed_n {
            match self.sense_seeds[si] {
                SenseSeed::Constant { kid, value } => {
                    self.set_port_hot(kid, PortSlot::new(0), value);
                }
                SenseSeed::SignalIn { kid } => {
                    let v = self.sense_values[usize::from(kid)];
                    self.set_port_hot(kid, PortSlot::new(0), v);
                }
                SenseSeed::OnStart { kid } => {
                    let ki = usize::from(kid);
                    let v = if !self.on_start_done[ki] {
                        self.on_start_done[ki] = true;
                        ONE
                    } else {
                        ZERO
                    };
                    self.set_port_hot(kid, PortSlot::new(0), v);
                }
            }
        }

        let topo_len = self.topo.len();
        for ti in 0..topo_len {
            let kid = self.topo[ti];
            let ki = usize::from(kid);
            if self.kind_tags[ki].is_sense() {
                continue;
            }
            self.gather_inputs(kid);
            self.eval_knot(kid);
        }
    }

    /// Copy inbound edge values into this knot's In ports.
    ///
    /// Uses a stack temp when fan-in > 2 so reads stay stable if a knot fans into
    /// itself across slots (max ports per knot is 8).
    fn gather_inputs(&mut self, kid: KnotId) {
        let ki = usize::from(kid);
        let start = self.inbound_off[ki] as usize;
        let end = self.inbound_off[ki + 1] as usize;
        let n = end - start;
        if n == 0 {
            return;
        }
        if n == 1 {
            let (f, fs, ts) = self.inbound_edges[start];
            let v = self.get_port_hot(f, fs);
            self.set_port_hot(kid, ts, v);
            return;
        }
        if n == 2 {
            let (f0, fs0, ts0) = self.inbound_edges[start];
            let (f1, fs1, ts1) = self.inbound_edges[start + 1];
            let v0 = self.get_port_hot(f0, fs0);
            let v1 = self.get_port_hot(f1, fs1);
            self.set_port_hot(kid, ts0, v0);
            self.set_port_hot(kid, ts1, v1);
            return;
        }
        let mut tmp: [(PortSlot, Signal); 8] = [(PortSlot::new(0), ZERO); 8];
        let n = n.min(8);
        for (i, &(f, fs, ts)) in self.inbound_edges[start..start + n].iter().enumerate() {
            tmp[i] = (ts, self.get_port_hot(f, fs));
        }
        for &(ts, v) in tmp.iter().take(n) {
            self.set_port_hot(kid, ts, v);
        }
    }

    fn eval_knot(&mut self, kid: KnotId) {
        let ki = usize::from(kid);
        let tag = self.kind_tags[ki];
        match tag {
            KindTag::Sense => {}
            KindTag::Not => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let o = if is_truthy(i) { ZERO } else { ONE };
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::And { arity } => {
                let mut ok = true;
                for s in 0..arity {
                    if !is_truthy(self.get_port_hot(kid, PortSlot(s))) {
                        ok = false;
                        break;
                    }
                }
                self.set_port_hot(kid, PortSlot(arity), if ok { ONE } else { ZERO });
            }
            KindTag::Or { arity } => {
                let mut ok = false;
                for s in 0..arity {
                    if is_truthy(self.get_port_hot(kid, PortSlot(s))) {
                        ok = true;
                        break;
                    }
                }
                self.set_port_hot(kid, PortSlot(arity), if ok { ONE } else { ZERO });
            }
            KindTag::RisingFromZero => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let prev = self.prev_in[ki];
                let o = if !is_truthy(prev) && is_truthy(i) {
                    ONE
                } else {
                    ZERO
                };
                self.prev_in[ki] = i;
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::Compare { op, rhs_const } => {
                let lhs = self.get_port_hot(kid, PortSlot::new(0));
                let rhs = match rhs_const {
                    Some(c) => c,
                    None => self.get_port_hot(kid, PortSlot::new(1)),
                };
                let o = if compare(op, lhs, rhs) { ONE } else { ZERO };
                self.set_port_hot(kid, PortSlot::new(2), o);
            }
            KindTag::Flag {
                priority,
                enable_toggle,
            } => {
                let set_l = self.get_port_hot(kid, PortSlot::new(0));
                let reset_l = self.get_port_hot(kid, PortSlot::new(1));
                let toggle_l = self.get_port_hot(kid, PortSlot::new(2));
                let set = is_truthy(set_l);
                let reset = is_truthy(reset_l);
                let toggle = enable_toggle && !is_truthy(self.prev_in[ki]) && is_truthy(toggle_l);
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
                self.set_port_hot(kid, PortSlot::new(3), if st { ONE } else { ZERO });
            }
            KindTag::Counter => {
                let inc = self.get_port_hot(kid, PortSlot::new(0));
                let dec = self.get_port_hot(kid, PortSlot::new(1));
                let reset = self.get_port_hot(kid, PortSlot::new(2));
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
                self.set_port_hot(kid, PortSlot::new(3), crate::from_count(self.counter[ki]));
            }
            KindTag::TimerPulseHold { ticks } => {
                let start = self.get_port_hot(kid, PortSlot::new(0));
                let prev = self.prev_in[ki];
                if !is_truthy(prev) && is_truthy(start) {
                    self.timer_left[ki] = ticks;
                }
                self.prev_in[ki] = start;
                if self.timer_left[ki] > 0 {
                    self.set_port_hot(kid, PortSlot::new(1), ONE);
                    self.timer_left[ki] -= 1;
                } else {
                    self.set_port_hot(kid, PortSlot::new(1), ZERO);
                }
            }
            KindTag::TimerFedCountdown { ticks } => {
                let feed = self.get_port_hot(kid, PortSlot::new(0));
                let prev = self.prev_in[ki];
                if is_truthy(feed) {
                    if !is_truthy(prev) {
                        self.timer_left[ki] = ticks;
                    }
                    if self.timer_left[ki] > 0 {
                        self.timer_left[ki] -= 1;
                    }
                    let active = if self.timer_left[ki] == 0 { ONE } else { ZERO };
                    self.set_port_hot(kid, PortSlot::new(1), active);
                } else {
                    self.timer_left[ki] = 0;
                    self.set_port_hot(kid, PortSlot::new(1), ZERO);
                }
                self.prev_in[ki] = feed;
            }
            KindTag::Delay { ticks } => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                if ticks == 0 {
                    self.set_port_hot(kid, PortSlot::new(1), i);
                } else {
                    let len = self.delay_len[ki] as usize;
                    let off = self.delay_off[ki] as usize;
                    let head = self.delay_head[ki] as usize;
                    let o = self.delay_buf[off + head];
                    self.delay_buf[off + head] = i;
                    let next = head + 1;
                    self.delay_head[ki] = if len.is_power_of_two() {
                        (next & (len - 1)) as u16
                    } else if next >= len {
                        0
                    } else {
                        next as u16
                    };
                    self.set_port_hot(kid, PortSlot::new(1), o);
                }
            }
            KindTag::CalcAdd { domain } => {
                let a = self.get_port_hot(kid, PortSlot::new(0));
                let b = self.get_port_hot(kid, PortSlot::new(1));
                let out = match domain {
                    SignalDomain::Count => count_add(a, b),
                    _ => crate::foundation::signal_ops::sat_add(a, b),
                };
                self.set_port_hot(kid, PortSlot::new(2), out);
            }
            KindTag::CalcSub { domain } => {
                let a = self.get_port_hot(kid, PortSlot::new(0));
                let b = self.get_port_hot(kid, PortSlot::new(1));
                let out = match domain {
                    SignalDomain::Count => count_sub(a, b),
                    _ => crate::foundation::signal_ops::sat_sub(a, b),
                };
                self.set_port_hot(kid, PortSlot::new(2), out);
            }
            KindTag::CalcMulLevel => {
                let a = self.get_port_hot(kid, PortSlot::new(0));
                let b = self.get_port_hot(kid, PortSlot::new(1));
                self.set_port_hot(kid, PortSlot::new(2), crate::foundation::signal_ops::mul(a, b));
            }
            KindTag::CalcMulCount => {
                let a = self.get_port_hot(kid, PortSlot::new(0));
                let b = self.get_port_hot(kid, PortSlot::new(1));
                self.set_port_hot(kid, PortSlot::new(2), count_mul(a, b));
            }
            KindTag::CalcDivLevel => {
                let a = self.get_port_hot(kid, PortSlot::new(0));
                let b = self.get_port_hot(kid, PortSlot::new(1));
                self.set_port_hot(kid, PortSlot::new(2), crate::foundation::signal_ops::div(a, b));
            }
            KindTag::CalcDivCount => {
                let a = self.get_port_hot(kid, PortSlot::new(0));
                let b = self.get_port_hot(kid, PortSlot::new(1));
                self.set_port_hot(kid, PortSlot::new(2), count_div(a, b));
            }
            KindTag::CalcDivLevelConst { divisor } => {
                let a = self.get_port_hot(kid, PortSlot::new(0));
                self.set_port_hot(
                    kid,
                    PortSlot::new(2),
                    crate::foundation::signal_ops::div(a, divisor),
                );
            }
            KindTag::CalcDivCountConst { divisor } => {
                let a = self.get_port_hot(kid, PortSlot::new(0));
                self.set_port_hot(kid, PortSlot::new(2), count_div(a, divisor));
            }
            KindTag::Abs { domain } => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let o = match domain {
                    SignalDomain::Count => count_abs(i),
                    _ => level_abs(i),
                };
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::Neg { domain } => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let o = match domain {
                    SignalDomain::Count => count_neg(i),
                    _ => level_neg(i),
                };
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::Map {
                domain,
                #[cfg(feature = "signal-f32")]
                degenerate,
                #[cfg(feature = "signal-f32")]
                in_min,
                #[cfg(feature = "signal-f32")]
                out_min,
                #[cfg(feature = "signal-f32")]
                inv_in_span,
                #[cfg(feature = "signal-f32")]
                out_span,
                #[cfg(feature = "signal-i32")]
                plan,
            } => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                #[cfg(feature = "signal-f32")]
                let o = map_linear_fast(
                    i,
                    domain,
                    degenerate,
                    in_min,
                    out_min,
                    inv_in_span,
                    out_span,
                );
                #[cfg(feature = "signal-i32")]
                let o = {
                    let _ = domain;
                    plan.map(i)
                };
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::Select => {
                let sel = self.get_port_hot(kid, PortSlot::new(0));
                let a = self.get_port_hot(kid, PortSlot::new(1));
                let b = self.get_port_hot(kid, PortSlot::new(2));
                let o = if is_truthy(sel) { b } else { a };
                self.set_port_hot(kid, PortSlot::new(3), o);
            }
            KindTag::Digitize {
                domain,
                degenerate,
                in_min,
                out_min,
                bin_scale,
                out_scale,
                last_f,
                steps,
                last,
                #[cfg(feature = "signal-i32")]
                den,
                #[cfg(feature = "signal-i32")]
                out_span,
            } => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let o = digitize_fast(
                    i,
                    domain,
                    degenerate,
                    in_min,
                    out_min,
                    bin_scale,
                    out_scale,
                    last_f,
                    steps,
                    last,
                    #[cfg(feature = "signal-i32")]
                    den,
                    #[cfg(feature = "signal-i32")]
                    out_span,
                );
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::Threshold {
                high,
                low,
                use_hysteresis,
            } => {
                let x = self.get_port_hot(kid, PortSlot::new(0));
                let prev = self.flag[ki];
                let mut latched = prev;
                if use_hysteresis {
                    if latched {
                        if x < low {
                            latched = false;
                        }
                    } else if x >= high {
                        latched = true;
                    }
                } else {
                    latched = x >= high;
                }
                self.flag[ki] = latched;
                self.set_port_hot(kid, PortSlot::new(1), if latched { ONE } else { ZERO });
                self.set_port_hot(
                    kid,
                    PortSlot::new(2),
                    if !prev && latched { ONE } else { ZERO },
                );
                self.set_port_hot(
                    kid,
                    PortSlot::new(3),
                    if prev && !latched { ONE } else { ZERO },
                );
            }
            tag @ (KindTag::RandomLevel {
                require_gate,
                min_wired,
                max_wired,
            }
            | KindTag::RandomCount {
                require_gate,
                min_wired,
                max_wired,
            }) => {
                // Unwired min/max ports default to ZERO / ONE (not cleared gather zeros).
                let min_v = if min_wired {
                    self.get_port_hot(kid, PortSlot::new(0))
                } else {
                    ZERO
                };
                let max_v = if max_wired {
                    self.get_port_hot(kid, PortSlot::new(1))
                } else {
                    match tag {
                        KindTag::RandomCount { .. } => crate::foundation::from_count(1),
                        _ => ONE,
                    }
                };
                let gate = self.get_port_hot(kid, PortSlot::new(2));
                let prev = self.prev_in[ki];
                let rising = !is_truthy(prev) && is_truthy(gate);
                let sample = if require_gate { rising } else { true };
                if sample {
                    let u = self.next_rng_u32();
                    let o = match tag {
                        KindTag::RandomCount { .. } => random_count_in_range(u, min_v, max_v),
                        _ => random_level_in_range(u, min_v, max_v),
                    };
                    self.set_port_hot(kid, PortSlot::new(3), o);
                    // Last sample is stashed in `counter` (shared rune storage).
                    #[cfg(feature = "signal-f32")]
                    {
                        self.counter[ki] = o.to_bits() as i32;
                    }
                    #[cfg(feature = "signal-i32")]
                    {
                        self.counter[ki] = o;
                    }
                } else {
                    #[cfg(feature = "signal-f32")]
                    {
                        self.set_port_hot(
                            kid,
                            PortSlot::new(3),
                            f32::from_bits(self.counter[ki] as u32),
                        );
                    }
                    #[cfg(feature = "signal-i32")]
                    {
                        self.set_port_hot(kid, PortSlot::new(3), self.counter[ki]);
                    }
                }
                self.prev_in[ki] = gate;
            }
            KindTag::SqrtLevel => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let o = level_sqrt(i);
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::SqrtCount => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let o = count_sqrt(i);
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::ConvertBoolToLevel | KindTag::ConvertIdentity => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                self.set_port_hot(kid, PortSlot::new(1), i);
            }
            KindTag::ConvertBoolToCount => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let o = crate::foundation::from_count(i32::from(is_truthy(i)));
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::ConvertLevelToBool | KindTag::ConvertCountToBool => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                self.set_port_hot(kid, PortSlot::new(1), if is_truthy(i) { ONE } else { ZERO });
            }
            KindTag::ConvertLevelToCount => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                self.set_port_hot(kid, PortSlot::new(1), level_to_count(i));
            }
            KindTag::ConvertCountToLevel => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                self.set_port_hot(kid, PortSlot::new(1), count_to_level(i));
            }
            KindTag::Xor => {
                let a = is_truthy(self.get_port_hot(kid, PortSlot::new(0)));
                let b = is_truthy(self.get_port_hot(kid, PortSlot::new(1)));
                self.set_port_hot(kid, PortSlot::new(2), if a ^ b { ONE } else { ZERO });
            }
            KindTag::FallingToZero => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let prev = self.prev_in[ki];
                let o = if is_truthy(prev) && !is_truthy(i) {
                    ONE
                } else {
                    ZERO
                };
                self.prev_in[ki] = i;
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::Change => {
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let prev = self.prev_in[ki];
                let o = if is_truthy(prev) != is_truthy(i) {
                    ONE
                } else {
                    ZERO
                };
                self.prev_in[ki] = i;
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::Clamp { min, max } => {
                debug_assert!(min <= max);
                let i = self.get_port_hot(kid, PortSlot::new(0));
                let o = if i < min {
                    min
                } else if i > max {
                    max
                } else {
                    i
                };
                self.set_port_hot(kid, PortSlot::new(1), o);
            }
            KindTag::SignalOut => {
                let v = self.get_port_hot(kid, PortSlot::new(0));
                if let Some(path) = self.knots[ki].path {
                    self.push_signal_out(path, v);
                }
            }
            KindTag::EmitCommand { enable_wired } => {
                let trig = self.get_port_hot(kid, PortSlot::new(0));
                // Unwired enable is treated as always on.
                let enable = if enable_wired {
                    self.get_port_hot(kid, PortSlot::new(1))
                } else {
                    ONE
                };
                let payload = self.get_port_hot(kid, PortSlot::new(2));
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

#[inline]
#[cfg(feature = "signal-f32")]
fn normalize_count(value: Signal) -> Signal {
    const MAX_COUNT: f32 = 2_147_483_520.0;
    if value.is_nan() {
        ZERO
    } else {
        value.trunc().clamp(i32::MIN as f32, MAX_COUNT)
    }
}

#[inline]
fn count_add(a: Signal, b: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        normalize_count(a + b)
    }
    #[cfg(feature = "signal-i32")]
    {
        a.saturating_add(b)
    }
}

#[inline]
fn count_sub(a: Signal, b: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        normalize_count(a - b)
    }
    #[cfg(feature = "signal-i32")]
    {
        a.saturating_sub(b)
    }
}

#[inline]
fn count_mul(a: Signal, b: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        normalize_count((a as f64 * b as f64) as f32)
    }
    #[cfg(feature = "signal-i32")]
    {
        a.saturating_mul(b)
    }
}

#[inline]
fn count_div(a: Signal, b: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        let a = a as i32;
        let b = b as i32;
        if b == 0 {
            0.0
        } else {
            normalize_count(a.checked_div(b).unwrap_or(i32::MAX) as f32)
        }
    }
    #[cfg(feature = "signal-i32")]
    {
        if b == 0 {
            0
        } else {
            a.checked_div(b).unwrap_or(i32::MAX)
        }
    }
}

#[inline]
fn level_to_count(value: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        let rounded = if value >= 0.0 {
            value + 0.5
        } else {
            value - 0.5
        };
        normalize_count(rounded)
    }
    #[cfg(feature = "signal-i32")]
    {
        let value = value as i64;
        let half = (ONE / 2) as i64;
        let rounded = if value >= 0 {
            (value + half) / (ONE as i64)
        } else {
            (value - half) / (ONE as i64)
        };
        rounded.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
}

#[inline]
fn count_abs(value: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        normalize_count(value.abs())
    }
    #[cfg(feature = "signal-i32")]
    {
        value.saturating_abs()
    }
}

#[inline]
fn count_neg(value: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        normalize_count(-value)
    }
    #[cfg(feature = "signal-i32")]
    {
        value.saturating_neg()
    }
}

#[inline]
fn level_abs(value: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        value.abs()
    }
    #[cfg(feature = "signal-i32")]
    {
        value.saturating_abs()
    }
}

#[inline]
fn level_neg(value: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        -value
    }
    #[cfg(feature = "signal-i32")]
    {
        value.saturating_neg()
    }
}

#[inline]
fn count_to_level(value: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        value
    }
    #[cfg(feature = "signal-i32")]
    {
        value.saturating_mul(ONE)
    }
}

#[cfg(feature = "signal-f32")]
#[inline]
fn map_linear_fast(
    i: Signal,
    domain: SignalDomain,
    degenerate: bool,
    in_min: Signal,
    out_min: Signal,
    inv_in_span: Signal,
    out_span: Signal,
) -> Signal {
    if degenerate {
        return out_min;
    }
    let t = ((i - in_min) * inv_in_span).clamp(0.0, 1.0);
    let mapped = out_min + t * out_span;
    if domain == SignalDomain::Count {
        normalize_count(mapped)
    } else {
        mapped
    }
}

#[cfg(test)]
pub(crate) fn map_linear_for_test(
    i: Signal,
    in_min: Signal,
    in_max: Signal,
    out_min: Signal,
    out_max: Signal,
) -> Signal {
    map_linear_for_domain_test(SignalDomain::Level, i, in_min, in_max, out_min, out_max)
}

#[cfg(test)]
pub(crate) fn map_linear_for_domain_test(
    domain: SignalDomain,
    i: Signal,
    in_min: Signal,
    in_max: Signal,
    out_min: Signal,
    out_max: Signal,
) -> Signal {
    match KindTag::map_precomputed(domain, in_min, in_max, out_min, out_max) {
        KindTag::Map {
            domain,
            #[cfg(feature = "signal-f32")]
            degenerate,
            #[cfg(feature = "signal-f32")]
            in_min,
            #[cfg(feature = "signal-f32")]
            out_min,
            #[cfg(feature = "signal-f32")]
            inv_in_span,
            #[cfg(feature = "signal-f32")]
            out_span,
            #[cfg(feature = "signal-i32")]
            plan,
        } => {
            #[cfg(feature = "signal-f32")]
            {
                map_linear_fast(
                    i,
                    domain,
                    degenerate,
                    in_min,
                    out_min,
                    inv_in_span,
                    out_span,
                )
            }
            #[cfg(feature = "signal-i32")]
            {
                let _ = domain;
                plan.map(i)
            }
        }
        _ => out_min,
    }
}

/// Quantize `i` into `steps` bins using bind-time scales (endpoints included).
#[inline]
#[allow(clippy::too_many_arguments)]
fn digitize_fast(
    i: Signal,
    domain: SignalDomain,
    degenerate: bool,
    in_min: Signal,
    out_min: Signal,
    bin_scale: Signal,
    out_scale: Signal,
    last_f: Signal,
    steps: u16,
    last: u16,
    #[cfg(feature = "signal-i32")] den: i64,
    #[cfg(feature = "signal-i32")] out_span: i64,
) -> Signal {
    if degenerate {
        return out_min;
    }
    #[cfg(feature = "signal-f32")]
    {
        let _ = (steps, last);
        let raw = (i - in_min) * bin_scale;
        let bin = raw.max(0.0).min(last_f) as u32;
        #[cfg(feature = "std")]
        let mapped = { out_scale.mul_add(bin as f32, out_min) };
        #[cfg(not(feature = "std"))]
        let mapped = { out_scale * bin as f32 + out_min };
        if domain == SignalDomain::Count {
            normalize_count(mapped)
        } else {
            mapped
        }
    }
    #[cfg(feature = "signal-i32")]
    {
        let _ = (domain, bin_scale, out_scale, last_f);
        let last_i = last as i64;
        let t = ((i as i64) - (in_min as i64)).clamp(0, den);
        let mut bin = t * (steps as i64) / den;
        if bin > last_i {
            bin = last_i;
        }
        (out_min as i64 + bin * out_span / last_i) as i32
    }
}

/// Public-for-tests digitize via the same precompute path as loom (shipped code).
#[cfg(test)]
pub(crate) fn digitize_for_test(
    i: Signal,
    steps: u16,
    in_min: Signal,
    in_max: Signal,
    out_min: Signal,
    out_max: Signal,
) -> Signal {
    digitize_for_domain_test(
        SignalDomain::Level,
        i,
        steps,
        in_min,
        in_max,
        out_min,
        out_max,
    )
}

#[cfg(test)]
pub(crate) fn digitize_for_domain_test(
    domain: SignalDomain,
    i: Signal,
    steps: u16,
    in_min: Signal,
    in_max: Signal,
    out_min: Signal,
    out_max: Signal,
) -> Signal {
    match KindTag::digitize_precomputed(domain, steps, in_min, in_max, out_min, out_max) {
        KindTag::Digitize {
            domain,
            degenerate,
            in_min,
            out_min,
            bin_scale,
            out_scale,
            last_f,
            steps,
            last,
            #[cfg(feature = "signal-i32")]
            den,
            #[cfg(feature = "signal-i32")]
            out_span,
        } => digitize_fast(
            i,
            domain,
            degenerate,
            in_min,
            out_min,
            bin_scale,
            out_scale,
            last_f,
            steps,
            last,
            #[cfg(feature = "signal-i32")]
            den,
            #[cfg(feature = "signal-i32")]
            out_span,
        ),
        _ => out_min,
    }
}

/// Map `u` into `[min_v, max_v]` (order-independent). i32 uses a wide product so
/// full-domain spans do not overflow intermediate mul.
fn random_level_in_range(u: u32, min_v: Signal, max_v: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        let t = (u as f32) / (u32::MAX as f32);
        let lo = min_v.min(max_v);
        let hi = min_v.max(max_v);
        lo + t * (hi - lo)
    }
    #[cfg(feature = "signal-i32")]
    {
        let lo = min_v.min(max_v) as i64;
        let hi = min_v.max(max_v) as i64;
        let span = hi - lo;
        if span <= 0 {
            return lo as i32;
        }
        let offset = ((u as u64) * (span as u64)) / (u32::MAX as u64);
        (lo + offset as i64) as i32
    }
}

fn random_count_in_range(u: u32, min_v: Signal, max_v: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    let (min_v, max_v) = (min_v as i32, max_v as i32);
    let lo = min_v.min(max_v) as i64;
    let hi = min_v.max(max_v) as i64;
    let span = hi - lo;
    if span <= 0 {
        return crate::foundation::from_count(lo as i32);
    }
    let offset = ((u as u64) * (span as u64)) / (u32::MAX as u64);
    crate::foundation::from_count((lo + offset as i64) as i32)
}

fn level_sqrt(i: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        if i <= 0.0 {
            0.0
        } else {
            sqrt_f32(i)
        }
    }
    #[cfg(feature = "signal-i32")]
    {
        if i <= 0 {
            0
        } else {
            isqrt_u64((i as u64) * (ONE as u64)) as i32
        }
    }
}

fn count_sqrt(i: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        if i <= 0.0 {
            0.0
        } else {
            (sqrt_f32(i) as i32) as f32
        }
    }
    #[cfg(feature = "signal-i32")]
    {
        if i <= 0 {
            0
        } else {
            isqrt_u64(i as u64) as i32
        }
    }
}

/// `no_std` square root for the float signal path.
#[cfg(feature = "signal-f32")]
#[inline]
fn sqrt_f32(value: f32) -> f32 {
    #[cfg(feature = "std")]
    {
        value.sqrt()
    }
    #[cfg(not(feature = "std"))]
    {
        libm::sqrtf(value)
    }
}

#[cfg(all(test, feature = "signal-f32"))]
mod sqrt_f32_tests {
    #[test]
    fn libm_matches_std_across_f32_magnitudes() {
        let values = [
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            1.0e-20,
            0.25,
            2.0,
            1.0e20,
            f32::MAX,
        ];

        for value in values {
            let expected = value.sqrt();
            let actual = libm::sqrtf(value);
            let tolerance = expected.abs() * (4.0 * f32::EPSILON);
            assert!(
                (actual - expected).abs() <= tolerance,
                "sqrt({value:e}): expected {expected:e}, got {actual:e}"
            );
        }
    }

    #[test]
    fn std_dispatch_preserves_native_sqrt_semantics() {
        for value in [f32::from_bits(1), 2.0, f32::MAX] {
            assert_eq!(super::sqrt_f32(value), value.sqrt());
        }
    }
}

/// Floor integer square root via restoring binary arithmetic.
///
/// This has a fixed, bounded shift/subtract loop and avoids the repeated wide
/// divisions Newton iteration would emit on 32-bit constrained targets.
#[cfg(feature = "signal-i32")]
#[inline]
fn isqrt_u64(mut n: u64) -> u64 {
    let mut result = 0u64;
    let mut bit = 1u64 << 62;

    while bit > n {
        bit >>= 2;
    }
    while bit != 0 {
        let trial = result + bit;
        if n >= trial {
            n -= trial;
            result = (result >> 1) + bit;
        } else {
            result >>= 1;
        }
        bit >>= 2;
    }
    result
}

#[cfg(all(test, feature = "signal-i32"))]
#[test]
fn isqrt_matches_dense_range_and_boundaries() {
    for n in 0u64..=65_536 {
        let root = isqrt_u64(n);
        assert!(root * root <= n, "isqrt({n}) overshot with {root}");
        assert!(
            (root + 1) * (root + 1) > n,
            "isqrt({n}) undershot with {root}"
        );
    }
    for k in 0i32..200 {
        let n = k * k;
        assert_eq!(isqrt_u64(n as u64), k as u64, "isqrt({n})");
        if k > 0 {
            assert_eq!(isqrt_u64((n - 1) as u64), (k - 1) as u64);
        }
    }
    assert_eq!(isqrt_u64(0), 0);
    assert_eq!(isqrt_u64(u64::MAX), u32::MAX as u64);
}

#[cfg(all(test, feature = "signal-i32"))]
mod random_i32_range_tests {
    use super::{random_count_in_range, random_level_in_range};

    #[test]
    fn full_i32_ranges_preserve_both_endpoints() {
        for range in [(i32::MIN, i32::MAX), (i32::MAX, i32::MIN)] {
            assert_eq!(random_level_in_range(0, range.0, range.1), i32::MIN);
            assert_eq!(random_level_in_range(u32::MAX, range.0, range.1), i32::MAX);
            assert_eq!(random_count_in_range(0, range.0, range.1), i32::MIN);
            assert_eq!(random_count_in_range(u32::MAX, range.0, range.1), i32::MAX);
        }
    }
}

#[cfg(test)]
mod domain_math_tests {
    use super::{
        count_abs, count_add, count_div, count_mul, count_neg, count_sqrt, count_sub,
        count_to_level, level_sqrt, level_to_count,
    };
    use crate::foundation::{from_count, from_level};

    #[test]
    fn count_arithmetic_is_integer_and_saturating() {
        assert_eq!(count_mul(from_count(6), from_count(7)), from_count(42));
        assert_eq!(
            count_mul(from_count(50_000), from_count(50_000)),
            from_count(i32::MAX)
        );
        assert_eq!(count_div(from_count(7), from_count(2)), from_count(3));
        assert_eq!(count_div(from_count(7), from_count(0)), from_count(0));
        assert_eq!(
            count_div(from_count(i32::MIN), from_count(-1)),
            from_count(i32::MAX)
        );
        assert_eq!(
            count_add(from_count(i32::MAX), from_count(1)),
            from_count(i32::MAX)
        );
        assert_eq!(
            count_sub(from_count(i32::MIN), from_count(1)),
            from_count(i32::MIN)
        );
        assert_eq!(count_abs(from_count(i32::MIN)), from_count(i32::MAX));
        assert_eq!(count_neg(from_count(i32::MIN)), from_count(i32::MAX));
    }

    #[test]
    fn numeric_conversions_round_and_saturate() {
        assert_eq!(level_to_count(from_level(2.5)), from_count(3));
        assert_eq!(level_to_count(from_level(-2.5)), from_count(-3));
        assert_eq!(count_to_level(from_count(2)), from_level(2.0));

        #[cfg(feature = "signal-i32")]
        assert_eq!(count_to_level(i32::MAX), i32::MAX);
    }

    #[test]
    fn sqrt_respects_level_and_count_representations() {
        assert_eq!(level_sqrt(from_level(0.25)), from_level(0.5));
        assert_eq!(count_sqrt(from_count(15)), from_count(3));
        assert_eq!(count_sqrt(from_count(-1)), from_count(0));
    }
}

#[cfg(test)]
mod digitize_tests {
    use super::{digitize_for_domain_test, digitize_for_test};
    use crate::foundation::{from_count, SignalDomain, ONE, ZERO};

    #[test]
    fn digitize_precompute_matches_endpoints_and_mids() {
        let steps = 4u16;
        let in0 = from_count(0);
        let in4 = from_count(4);
        let o0 = from_count(0);
        let o30 = from_count(30);
        assert_eq!(
            digitize_for_test(from_count(0), steps, in0, in4, o0, o30),
            from_count(0)
        );
        assert_eq!(
            digitize_for_test(from_count(1), steps, in0, in4, o0, o30),
            from_count(10)
        );
        assert_eq!(
            digitize_for_test(from_count(2), steps, in0, in4, o0, o30),
            from_count(20)
        );
        assert_eq!(
            digitize_for_test(from_count(3), steps, in0, in4, o0, o30),
            from_count(30)
        );
        assert_eq!(
            digitize_for_test(from_count(4), steps, in0, in4, o0, o30),
            from_count(30)
        );
        assert_eq!(
            digitize_for_test(ONE, 1, ZERO, ONE, from_count(7), from_count(9)),
            from_count(7)
        );
    }

    #[test]
    fn count_digitize_truncates_non_divisible_ranges() {
        assert_eq!(
            digitize_for_domain_test(
                SignalDomain::Count,
                from_count(1),
                4,
                from_count(0),
                from_count(3),
                from_count(0),
                from_count(10),
            ),
            from_count(3)
        );
    }
}

#[cfg(test)]
mod map_tests {
    use super::{map_linear_for_domain_test, map_linear_for_test};
    use crate::foundation::{from_count, SignalDomain, ONE, ZERO};

    #[cfg(feature = "signal-i32")]
    use crate::runtime_impl::kind_tag::{I32MapPlan, KindTag};

    #[test]
    fn map_precompute_endpoints_mid_and_degenerate() {
        let i0 = from_count(0);
        let i4 = from_count(4);
        let o0 = from_count(0);
        let o40 = from_count(40);
        assert_eq!(map_linear_for_test(from_count(0), i0, i4, o0, o40), o0);
        assert_eq!(
            map_linear_for_test(from_count(2), i0, i4, o0, o40),
            from_count(20)
        );
        assert_eq!(map_linear_for_test(from_count(4), i0, i4, o0, o40), o40);
        assert_eq!(map_linear_for_test(from_count(8), i0, i4, o0, o40), o40);
        assert_eq!(map_linear_for_test(from_count(-2), i0, i4, o0, o40), o0);
        assert_eq!(
            map_linear_for_test(ONE, from_count(3), from_count(3), from_count(7), o40),
            from_count(7)
        );
        let mid = map_linear_for_test(
            #[cfg(feature = "signal-f32")]
            {
                0.5
            },
            #[cfg(feature = "signal-i32")]
            {
                ONE / 2
            },
            ZERO,
            ONE,
            ZERO,
            ONE,
        );
        #[cfg(feature = "signal-f32")]
        assert!((mid - 0.5).abs() < 1e-5);
        #[cfg(feature = "signal-i32")]
        assert_eq!(mid, ONE / 2);
    }

    #[test]
    fn count_map_truncates_non_divisible_ranges() {
        assert_eq!(
            map_linear_for_domain_test(
                SignalDomain::Count,
                from_count(1),
                from_count(0),
                from_count(3),
                from_count(0),
                from_count(10),
            ),
            from_count(3)
        );
    }

    #[cfg(feature = "signal-i32")]
    #[test]
    fn map_full_i32_range_uses_wide_intermediates() {
        assert_eq!(
            map_linear_for_test(i32::MIN, i32::MIN, i32::MAX, i32::MIN, i32::MAX),
            i32::MIN
        );
        assert_eq!(
            map_linear_for_test(i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX),
            i32::MAX
        );
        assert_eq!(
            map_linear_for_test(0, i32::MIN, i32::MAX, i32::MIN, i32::MAX),
            0
        );

        assert_eq!(
            map_linear_for_test(i32::MIN, i32::MIN, i32::MAX, i32::MAX, i32::MIN),
            i32::MAX
        );
        assert_eq!(
            map_linear_for_test(i32::MAX, i32::MIN, i32::MAX, i32::MAX, i32::MIN),
            i32::MIN
        );
        assert_eq!(
            map_linear_for_test(0, i32::MIN, i32::MAX, i32::MAX, i32::MIN),
            -1
        );
    }

    #[cfg(feature = "signal-i32")]
    #[test]
    fn map_full_i32_output_range_clamps_inputs_outside_range() {
        assert_eq!(
            map_linear_for_test(i32::MIN, -1, 1, i32::MIN, i32::MAX),
            i32::MIN
        );
        assert_eq!(
            map_linear_for_test(i32::MAX, -1, 1, i32::MIN, i32::MAX),
            i32::MAX
        );
    }

    #[cfg(feature = "signal-i32")]
    fn reference_i32_map(input: i32, in_min: i32, in_max: i32, out_min: i32, out_max: i32) -> i32 {
        let den = (in_max as i128) - (in_min as i128);
        if den == 0 {
            return out_min;
        }
        let t = ((input as i128) - (in_min as i128)).clamp(0, den);
        ((out_min as i128) + t * ((out_max as i128) - (out_min as i128)) / den) as i32
    }

    #[cfg(feature = "signal-i32")]
    #[test]
    fn map_i32_uses_specialized_bind_plans() {
        let plan = |in_min, in_max, out_min, out_max| match KindTag::map_precomputed(
            SignalDomain::Count,
            in_min,
            in_max,
            out_min,
            out_max,
        ) {
            KindTag::Map { plan, .. } => plan,
            _ => unreachable!("map precompute must return a Map tag"),
        };

        assert!(matches!(plan(3, 3, 7, 9), I32MapPlan::Constant { .. }));
        assert!(matches!(plan(-8, 8, 5, 21), I32MapPlan::Unit { .. }));
        assert!(matches!(plan(0, 8, 0, 16), I32MapPlan::Scale { .. }));
        assert!(matches!(plan(0, 8, 0, 4), I32MapPlan::Shift { .. }));
        assert!(matches!(plan(0, 10, 0, 3), I32MapPlan::Divide { .. }));
    }

    #[cfg(feature = "signal-i32")]
    #[test]
    fn map_i32_matches_wide_reference_across_edge_and_seeded_ranges() {
        let cases = [
            (i32::MIN, i32::MAX, i32::MIN, i32::MAX),
            (i32::MIN, i32::MAX, i32::MAX, i32::MIN),
            (-65_536, 65_536, 0, 65_536),
            (0, 8, 0, 16),
            (0, 8, 0, 4),
            (0, 10, -37, 997),
            (3, 3, -5, 42),
        ];
        for (in_min, in_max, out_min, out_max) in cases {
            for input in [
                i32::MIN,
                in_min.saturating_sub(1),
                in_min,
                0,
                in_max,
                in_max.saturating_add(1),
                i32::MAX,
            ] {
                assert_eq!(
                    map_linear_for_domain_test(
                        SignalDomain::Count,
                        input,
                        in_min,
                        in_max,
                        out_min,
                        out_max,
                    ),
                    reference_i32_map(input, in_min, in_max, out_min, out_max),
                    "input={input}, in={in_min}..{in_max}, out={out_min}..{out_max}"
                );
            }
        }

        let mut state = 0x9E37_79B9u32;
        for _ in 0..512 {
            let next = |state: &mut u32| {
                *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *state as i32
            };
            let a = next(&mut state);
            let b = next(&mut state);
            let (in_min, in_max) = if a <= b { (a, b) } else { (b, a) };
            let out_min = next(&mut state);
            let out_max = next(&mut state);
            let input = next(&mut state);
            assert_eq!(
                map_linear_for_domain_test(
                    SignalDomain::Count,
                    input,
                    in_min,
                    in_max,
                    out_min,
                    out_max,
                ),
                reference_i32_map(input, in_min, in_max, out_min, out_max),
            );
        }
    }
}
