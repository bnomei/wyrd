use wyrd_core::{
    is_truthy, CompareOp, FlagPriority, KnotId, PortSlot, Result, Signal, ONE, ZERO,
};
use wyrd_graph::Weave;

use crate::bind::{Runtime, SenseSeed};
use crate::kind_tag::KindTag;

impl Runtime {
    /// One settle pass. Never panics. No topology alloc after bind.
    pub fn loom(&mut self, _weave: &Weave) -> Result<()> {
        // 1. Zero all In ports via flat bind-time index list (no per-knot Vec walk).
        for &idx in &self.clear_port_idx {
            debug_assert!(idx < self.port_vals.len());
            self.port_vals[idx] = ZERO;
        }

        // 2. Seed Sense outputs only (bind-time list — no full-knot scan).
        let seed_n = self.sense_seeds.len();
        for si in 0..seed_n {
            match self.sense_seeds[si] {
                SenseSeed::Constant { kid, value } => {
                    self.set_port_hot(kid, PortSlot(0), value);
                }
                SenseSeed::SignalIn { kid } => {
                    let v = self.sense_values[kid.0 as usize];
                    self.set_port_hot(kid, PortSlot(0), v);
                }
                SenseSeed::OnStart { kid } => {
                    let ki = kid.0 as usize;
                    let v = if !self.on_start_done[ki] {
                        self.on_start_done[ki] = true;
                        ONE
                    } else {
                        ZERO
                    };
                    self.set_port_hot(kid, PortSlot(0), v);
                }
            }
        }

        // 3. Topo eval — skip Sense (already seeded; no inputs to gather).
        let topo_len = self.topo.len();
        for ti in 0..topo_len {
            let kid = self.topo[ti];
            let ki = kid.0 as usize;
            if self.kind_tags[ki].is_sense() {
                continue;
            }
            self.gather_inputs(kid);
            self.eval_knot(kid);
        }

        Ok(())
    }

    fn gather_inputs(&mut self, kid: KnotId) {
        let ki = kid.0 as usize;
        let start = self.inbound_off[ki] as usize;
        let end = self.inbound_off[ki + 1] as usize;
        let n = end - start;
        if n == 0 {
            return;
        }
        // Fast paths: chain knots usually have 1 inbound edge (no stack tmp).
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
        // Stack buffer: max ports per knot is 8.
        let mut tmp: [(PortSlot, Signal); 8] = [(PortSlot(0), ZERO); 8];
        let n = n.min(8);
        for i in 0..n {
            let (f, fs, ts) = self.inbound_edges[start + i];
            tmp[i] = (ts, self.get_port_hot(f, fs));
        }
        for i in 0..n {
            self.set_port_hot(kid, tmp[i].0, tmp[i].1);
        }
    }

    fn eval_knot(&mut self, kid: KnotId) {
        let ki = kid.0 as usize;
        let tag = self.kind_tags[ki];
        match tag {
            KindTag::Sense => {}
            KindTag::Not => {
                let i = self.get_port_hot(kid, PortSlot(0));
                let o = if is_truthy(i) { ZERO } else { ONE };
                self.set_port_hot(kid, PortSlot(1), o);
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
                let i = self.get_port_hot(kid, PortSlot(0));
                let prev = self.prev_in[ki];
                let o = if !is_truthy(prev) && is_truthy(i) {
                    ONE
                } else {
                    ZERO
                };
                self.prev_in[ki] = i;
                self.set_port_hot(kid, PortSlot(1), o);
            }
            KindTag::Compare { op, rhs_const } => {
                let lhs = self.get_port_hot(kid, PortSlot(0));
                let rhs = match rhs_const {
                    Some(c) => c,
                    None => self.get_port_hot(kid, PortSlot(1)),
                };
                let o = if compare(op, lhs, rhs) { ONE } else { ZERO };
                self.set_port_hot(kid, PortSlot(2), o);
            }
            KindTag::Flag {
                priority,
                enable_toggle,
            } => {
                let set_l = self.get_port_hot(kid, PortSlot(0));
                let reset_l = self.get_port_hot(kid, PortSlot(1));
                let toggle_l = self.get_port_hot(kid, PortSlot(2));
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
                self.set_port_hot(kid, PortSlot(3), if st { ONE } else { ZERO });
            }
            KindTag::Counter => {
                let inc = self.get_port_hot(kid, PortSlot(0));
                let dec = self.get_port_hot(kid, PortSlot(1));
                let reset = self.get_port_hot(kid, PortSlot(2));
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
                // Whole-count Signal: i32 from_count is identity; f32 casts.
                self.set_port_hot(kid, PortSlot(3), crate::from_count(self.counter[ki]));
            }
            KindTag::TimerPulseHold { ticks } => {
                let start = self.get_port_hot(kid, PortSlot(0));
                let prev = self.prev_in[ki];
                if !is_truthy(prev) && is_truthy(start) {
                    self.timer_left[ki] = ticks;
                }
                self.prev_in[ki] = start;
                if self.timer_left[ki] > 0 {
                    self.set_port_hot(kid, PortSlot(1), ONE);
                    self.timer_left[ki] -= 1;
                } else {
                    self.set_port_hot(kid, PortSlot(1), ZERO);
                }
            }
            KindTag::TimerFedCountdown { ticks } => {
                let feed = self.get_port_hot(kid, PortSlot(0));
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
                    self.set_port_hot(kid, PortSlot(1), active);
                } else {
                    self.timer_left[ki] = 0;
                    self.set_port_hot(kid, PortSlot(1), ZERO);
                }
                self.prev_in[ki] = feed;
            }
            KindTag::Delay { ticks } => {
                let i = self.get_port_hot(kid, PortSlot(0));
                if ticks == 0 {
                    self.set_port_hot(kid, PortSlot(1), i);
                } else {
                    let len = self.delay_len[ki] as usize;
                    let off = self.delay_off[ki] as usize;
                    let head = self.delay_head[ki] as usize;
                    let o = self.delay_buf[off + head];
                    self.delay_buf[off + head] = i;
                    // Power-of-two ring: mask; else branch wrap.
                    let next = head + 1;
                    self.delay_head[ki] = if len.is_power_of_two() {
                        (next & (len - 1)) as u16
                    } else if next >= len {
                        0
                    } else {
                        next as u16
                    };
                    self.set_port_hot(kid, PortSlot(1), o);
                }
            }
            KindTag::CalcAdd => {
                let a = self.get_port_hot(kid, PortSlot(0));
                let b = self.get_port_hot(kid, PortSlot(1));
                self.set_port_hot(kid, PortSlot(2), wyrd_core::signal_ops::sat_add(a, b));
            }
            KindTag::CalcSub => {
                let a = self.get_port_hot(kid, PortSlot(0));
                let b = self.get_port_hot(kid, PortSlot(1));
                self.set_port_hot(kid, PortSlot(2), wyrd_core::signal_ops::sat_sub(a, b));
            }
            KindTag::CalcMul => {
                let a = self.get_port_hot(kid, PortSlot(0));
                let b = self.get_port_hot(kid, PortSlot(1));
                self.set_port_hot(kid, PortSlot(2), wyrd_core::signal_ops::mul(a, b));
            }
            KindTag::CalcDiv => {
                let a = self.get_port_hot(kid, PortSlot(0));
                let b = self.get_port_hot(kid, PortSlot(1));
                self.set_port_hot(kid, PortSlot(2), wyrd_core::signal_ops::div(a, b));
            }
            KindTag::CalcDivConst { divisor } => {
                let a = self.get_port_hot(kid, PortSlot(0));
                self.set_port_hot(kid, PortSlot(2), wyrd_core::signal_ops::div(a, divisor));
            }
            KindTag::Abs => {
                let i = self.get_port_hot(kid, PortSlot(0));
                #[cfg(feature = "signal-f32")]
                let o = if i < 0.0 { -i } else { i };
                #[cfg(feature = "signal-i32")]
                let o = i.saturating_abs();
                self.set_port_hot(kid, PortSlot(1), o);
            }
            KindTag::Neg => {
                let i = self.get_port_hot(kid, PortSlot(0));
                #[cfg(feature = "signal-f32")]
                let o = -i;
                #[cfg(feature = "signal-i32")]
                let o = i.saturating_neg();
                self.set_port_hot(kid, PortSlot(1), o);
            }
            KindTag::Map {
                degenerate,
                in_min,
                out_min,
                inv_in_span,
                out_span,
                #[cfg(feature = "signal-i32")]
                    den,
                #[cfg(feature = "signal-i32")]
                    out_span_i64,
            } => {
                let i = self.get_port_hot(kid, PortSlot(0));
                let o = map_linear_fast(
                    i,
                    degenerate,
                    in_min,
                    out_min,
                    inv_in_span,
                    out_span,
                    #[cfg(feature = "signal-i32")]
                    den,
                    #[cfg(feature = "signal-i32")]
                    out_span_i64,
                );
                self.set_port_hot(kid, PortSlot(1), o);
            }
            KindTag::Select => {
                let sel = self.get_port_hot(kid, PortSlot(0));
                let a = self.get_port_hot(kid, PortSlot(1));
                let b = self.get_port_hot(kid, PortSlot(2));
                let o = if is_truthy(sel) { b } else { a };
                self.set_port_hot(kid, PortSlot(3), o);
            }
            KindTag::Digitize {
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
                let i = self.get_port_hot(kid, PortSlot(0));
                let o = digitize_fast(
                    i,
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
                self.set_port_hot(kid, PortSlot(1), o);
            }
            KindTag::Threshold {
                high,
                low,
                use_hysteresis,
            } => {
                let x = self.get_port_hot(kid, PortSlot(0));
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
                self.set_port_hot(kid, PortSlot(1), if latched { ONE } else { ZERO });
                self.set_port_hot(
                    kid,
                    PortSlot(2),
                    if !prev && latched { ONE } else { ZERO },
                );
                self.set_port_hot(
                    kid,
                    PortSlot(3),
                    if prev && !latched { ONE } else { ZERO },
                );
            }
            KindTag::Random {
                require_gate,
                min_wired,
                max_wired,
            } => {
                // Ports: min(0), max(1), gate(2), out(3). Unconnected min→ZERO, max→ONE.
                let min_v = if min_wired {
                    self.get_port_hot(kid, PortSlot(0))
                } else {
                    ZERO
                };
                let max_v = if max_wired {
                    self.get_port_hot(kid, PortSlot(1))
                } else {
                    ONE
                };
                let gate = self.get_port_hot(kid, PortSlot(2));
                let prev = self.prev_in[ki];
                let rising = !is_truthy(prev) && is_truthy(gate);
                let sample = if require_gate { rising } else { true };
                if sample {
                    let u = self.next_rng_u32();
                    let o = random_in_range(u, min_v, max_v);
                    self.set_port_hot(kid, PortSlot(3), o);
                    #[cfg(feature = "signal-f32")]
                    {
                        self.counter[ki] = o.to_bits() as i32;
                    }
                    #[cfg(feature = "signal-i32")]
                    {
                        self.counter[ki] = o;
                    }
                } else {
                    // Hold last sample in `counter` storage.
                    #[cfg(feature = "signal-f32")]
                    {
                        self.set_port_hot(kid, PortSlot(3), f32::from_bits(self.counter[ki] as u32));
                    }
                    #[cfg(feature = "signal-i32")]
                    {
                        self.set_port_hot(kid, PortSlot(3), self.counter[ki]);
                    }
                }
                self.prev_in[ki] = gate;
            }
            KindTag::Sqrt => {
                let i = self.get_port_hot(kid, PortSlot(0));
                let o = signal_sqrt(i);
                self.set_port_hot(kid, PortSlot(1), o);
            }
            KindTag::Xor => {
                let a = is_truthy(self.get_port_hot(kid, PortSlot(0)));
                let b = is_truthy(self.get_port_hot(kid, PortSlot(1)));
                self.set_port_hot(kid, PortSlot(2), if a ^ b { ONE } else { ZERO });
            }
            KindTag::FallingToZero => {
                let i = self.get_port_hot(kid, PortSlot(0));
                let prev = self.prev_in[ki];
                let o = if is_truthy(prev) && !is_truthy(i) {
                    ONE
                } else {
                    ZERO
                };
                self.prev_in[ki] = i;
                self.set_port_hot(kid, PortSlot(1), o);
            }
            KindTag::Change => {
                let i = self.get_port_hot(kid, PortSlot(0));
                let prev = self.prev_in[ki];
                let o = if is_truthy(prev) != is_truthy(i) {
                    ONE
                } else {
                    ZERO
                };
                self.prev_in[ki] = i;
                self.set_port_hot(kid, PortSlot(1), o);
            }
            KindTag::Clamp { min, max } => {
                // validate rejects min > max before bind.
                debug_assert!(min <= max);
                let i = self.get_port_hot(kid, PortSlot(0));
                let o = if i < min {
                    min
                } else if i > max {
                    max
                } else {
                    i
                };
                self.set_port_hot(kid, PortSlot(1), o);
            }
            KindTag::SignalOut => {
                let v = self.get_port_hot(kid, PortSlot(0));
                if let Some(path) = self.knots[ki].path {
                    self.push_signal_out(path, v);
                }
            }
            KindTag::EmitCommand { enable_wired } => {
                let trig = self.get_port_hot(kid, PortSlot(0));
                // Optional enable: unconnected → treated as ONE (enabled).
                let enable = if enable_wired {
                    self.get_port_hot(kid, PortSlot(1))
                } else {
                    ONE
                };
                let payload = self.get_port_hot(kid, PortSlot(2));
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
fn map_linear_fast(
    i: Signal,
    degenerate: bool,
    in_min: Signal,
    out_min: Signal,
    inv_in_span: Signal,
    out_span: Signal,
    #[cfg(feature = "signal-i32")] den: i64,
    #[cfg(feature = "signal-i32")] out_span_i64: i64,
) -> Signal {
    if degenerate {
        return out_min;
    }
    #[cfg(feature = "signal-f32")]
    {
        let t = ((i - in_min) * inv_in_span).clamp(0.0, 1.0);
        out_min + t * out_span
    }
    #[cfg(feature = "signal-i32")]
    {
        let _ = (inv_in_span, out_span);
        let t = ((i as i64) - (in_min as i64)).clamp(0, den);
        (out_min as i64 + t * out_span_i64 / den) as i32
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
    match KindTag::map_precomputed(in_min, in_max, out_min, out_max) {
        KindTag::Map {
            degenerate,
            in_min,
            out_min,
            inv_in_span,
            out_span,
            #[cfg(feature = "signal-i32")]
                den,
            #[cfg(feature = "signal-i32")]
                out_span_i64,
        } => map_linear_fast(
            i,
            degenerate,
            in_min,
            out_min,
            inv_in_span,
            out_span,
            #[cfg(feature = "signal-i32")]
            den,
            #[cfg(feature = "signal-i32")]
            out_span_i64,
        ),
        _ => out_min,
    }
}

/// Quantize `i` into `steps` bins over in range, map to out range (endpoints included).
/// Digitize using bind-time precomputed scales (hot path).
#[inline]
fn digitize_fast(
    i: Signal,
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
        // Integer bin in [0, last]; endpoint (raw≥steps) clamps via last_f.
        let raw = (i - in_min) * bin_scale;
        let bin = raw.max(0.0).min(last_f) as u32;
        out_scale.mul_add(bin as f32, out_min)
    }
    #[cfg(feature = "signal-i32")]
    {
        let _ = (bin_scale, out_scale, last_f);
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
    match KindTag::digitize_precomputed(steps, in_min, in_max, out_min, out_max) {
        KindTag::Digitize {
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

fn random_in_range(u: u32, min_v: Signal, max_v: Signal) -> Signal {
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
        // u128 product avoids overflow when span is large (full Signal domain).
        let offset = ((u as u128) * (span as u128)) / (u32::MAX as u128);
        (lo + offset as i64) as i32
    }
}

fn signal_sqrt(i: Signal) -> Signal {
    #[cfg(feature = "signal-f32")]
    {
        if i <= 0.0 {
            0.0
        } else {
            // Hardware/core sqrt when available (same IEEE result as libm::sqrtf on desktop).
            i.sqrt()
        }
    }
    #[cfg(feature = "signal-i32")]
    {
        isqrt_i32(i)
    }
}

/// Integer square root (floor). Newton iteration until convergence.
#[cfg(feature = "signal-i32")]
#[inline]
fn isqrt_i32(n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    if n < 4 {
        return 1;
    }
    // Seed: roughly 2^ceil(log2(n)/2)
    let z = n as u32;
    let mut x = 1i32 << ((32 - z.leading_zeros() + 1) / 2);
    loop {
        let y = ((x as i64) + (n as i64) / (x as i64)) / 2;
        let y = y as i32;
        if y >= x {
            return x;
        }
        x = y;
    }
}

#[cfg(all(test, feature = "signal-i32"))]
#[test]
fn isqrt_matches_perfect_squares() {
    for k in 0i32..200 {
        let n = k * k;
        assert_eq!(isqrt_i32(n), k, "isqrt({n})");
        if k > 0 {
            assert_eq!(isqrt_i32(n - 1), k - 1);
        }
    }
    assert_eq!(isqrt_i32(0), 0);
    assert_eq!(isqrt_i32(-3), 0);
}

#[cfg(test)]
mod digitize_tests {
    use super::digitize_for_test;
    use wyrd_core::{from_count, ONE, ZERO};

    #[test]
    fn digitize_precompute_matches_endpoints_and_mids() {
        // steps=4 over count 0..4 → out 0,10,20,30
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
        // Degenerate steps=1
        assert_eq!(
            digitize_for_test(ONE, 1, ZERO, ONE, from_count(7), from_count(9)),
            from_count(7)
        );
    }
}

#[cfg(test)]
mod map_tests {
    use super::map_linear_for_test;
    use wyrd_core::{from_count, ONE, ZERO};

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
        // Outside clamp
        assert_eq!(map_linear_for_test(from_count(8), i0, i4, o0, o40), o40);
        assert_eq!(map_linear_for_test(from_count(-2), i0, i4, o0, o40), o0);
        // Zero in-span → out_min
        assert_eq!(
            map_linear_for_test(ONE, from_count(3), from_count(3), from_count(7), o40),
            from_count(7)
        );
        // ZERO..ONE identity-ish for out ZERO..ONE at mid
        let mid = map_linear_for_test(
            #[cfg(feature = "signal-f32")]
            {
                0.5
            },
            #[cfg(feature = "signal-i32")]
            {
                // Q16 half of ONE if ONE is 1<<16
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
}
