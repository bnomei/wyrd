//! Bind: turn a validated [`Weave`] into a dense, executable [`Runtime`].
//!
//! Interns host paths and command names, builds CSR inbound edges, topo order,
//! kind-dispatch tags, delay rings, and sense seed lists so [`Runtime::loom`]
//! allocates no topology after bind. Host resolves `sense_id` / `path_id` once
//! at setup; the hot path uses only dense ids.

use std::collections::BTreeMap;
use std::string::String;
use std::vec;
use std::vec::Vec;

use wyrd_core::{
    port_slot, ports_of, CalcOp, CmdId, HostPathId, HostTime, KnotId, KnotKind, PortDir, PortSlot,
    Seed, SenseId, Signal, ZERO,
};
use wyrd_graph::{validate, Budget, Weave};

use crate::error::{BindError, HandleError};
use crate::outbox::{Emit, Outbox, PortWriter, SignalOutSample};

/// Bind-time sense seed entry — only Sense knots, so loom need not scan all knots.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SenseSeed {
    Constant { kid: KnotId, value: Signal },
    SignalIn { kid: KnotId },
    OnStart { kid: KnotId },
}

/// Bind-time options (sandbox / host policy).
#[derive(Clone, Debug)]
pub struct BindOpts {
    pub seed: Option<Seed>,
    /// Hard cap on EmitCommand outbox entries per loom (default 8).
    /// Further emits in the same tick are dropped (no panic).
    pub max_emits_per_tick: u16,
    /// Validate budget (default matches [`Budget::default`]).
    pub budget: Budget,
}

impl Default for BindOpts {
    fn default() -> Self {
        Self {
            seed: None,
            max_emits_per_tick: 8,
            budget: Budget::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedKnot {
    pub(crate) kind: KnotKind,
    /// For SignalOut / Emit after intern
    pub(crate) path: Option<HostPathId>,
    pub(crate) cmd: Option<CmdId>,
}

/// Bound runtime: dense buffers, topo order, intern tables, stateful rune storage.
///
/// Sole executable artifact after bind. Sample senses through [`PortWriter`],
/// settle with [`Self::loom`], then read [`Self::outbox`].
pub struct Runtime {
    pub(crate) knots: Vec<ResolvedKnot>,
    /// Author name → KnotId
    name_to_id: BTreeMap<String, KnotId>,
    /// KnotId → author name
    #[allow(dead_code)]
    id_to_name: Vec<String>,
    path_names: Vec<String>,
    cmd_names: Vec<String>,
    /// Threads: (from_knot, from_slot, to_knot, to_slot) — retained for debug; loom uses `inbound`.
    #[allow(dead_code)]
    pub(crate) threads: Vec<(KnotId, PortSlot, KnotId, PortSlot)>,
    /// CSR inbound: edges in `inbound_edges[inbound_off[ki]..inbound_off[ki+1]]`.
    /// Each edge is (from_knot, from_slot, to_slot).
    pub(crate) inbound_off: Vec<u32>,
    pub(crate) inbound_edges: Vec<(KnotId, PortSlot, PortSlot)>,
    /// Absolute `port_vals` indices of In ports to zero each loom (flat, bind-sized).
    pub(crate) clear_port_idx: Vec<usize>,
    /// Per-knot input slots (retained for diagnostics; clear uses `clear_port_idx`).
    #[allow(dead_code)]
    pub(crate) input_slots: Vec<Vec<PortSlot>>,
    pub(crate) topo: Vec<KnotId>,
    /// Bind-time kind dispatch tags (one per knot; no per-tick from_kind).
    pub(crate) kind_tags: Vec<crate::kind_tag::KindTag>,
    /// Only Constant / SignalIn / OnStart — loom seeds these without scanning all knots.
    pub(crate) sense_seeds: Vec<SenseSeed>,
    /// Host-fed sense outputs (SignalIn).
    pub(crate) sense_values: Vec<Signal>,
    /// Port value store: indexed by (knot_idx * MAX_PORTS + slot)
    pub(crate) port_vals: Vec<Signal>,
    pub(crate) max_ports: usize,
    /// Knot state for stateful runes
    pub(crate) prev_in: Vec<Signal>,
    pub(crate) prev_dec: Vec<Signal>,
    pub(crate) counter: Vec<i32>,
    pub(crate) flag: Vec<bool>,
    pub(crate) timer_left: Vec<u16>,
    pub(crate) on_start_done: Vec<bool>,
    /// Delay ring: flat buffer + per-knot (offset, len, head).
    pub(crate) delay_buf: Vec<Signal>,
    pub(crate) delay_off: Vec<u16>,
    pub(crate) delay_len: Vec<u16>,
    pub(crate) delay_head: Vec<u16>,
    out_signals: Vec<SignalOutSample>,
    out_emits: Vec<Emit>,
    max_emits_per_tick: u16,
    tick: u64,
    /// Deterministic xorshift state for Random knots (never zero).
    pub(crate) rng: u64,
    /// `fnv1a64(weave.id)` mixed into seeds at bind and [`Self::reseed`].
    seed_mix: u64,
}

const MAX_PORTS: usize = 8;

fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

impl Runtime {
    /// Validate and consume a weave into dense executable state.
    ///
    /// # Errors
    ///
    /// Returns [`BindError`] when budget validation fails, dense capacity is
    /// exceeded, a port cannot be resolved, or topo order cannot be built.
    pub fn bind(weave: Weave, opts: BindOpts) -> Result<Self, BindError> {
        let weave_id = String::from(weave.id());
        validate(&weave, &opts.budget).map_err(|source| BindError::InvalidWeave {
            weave_id: weave_id.clone(),
            source,
        })?;

        let mut name_to_id = BTreeMap::new();
        let mut id_to_name = Vec::new();
        let mut path_names = Vec::new();
        let mut cmd_names = Vec::new();
        let mut path_index: BTreeMap<String, HostPathId> = BTreeMap::new();
        let mut cmd_index: BTreeMap<String, CmdId> = BTreeMap::new();

        let mut knots = Vec::with_capacity(weave.knots().len());
        for (i, k) in weave.knots().iter().enumerate() {
            let id = KnotId::try_from(i).map_err(|_| BindError::CapacityExceeded {
                weave_id: weave_id.clone(),
                resource: "knot",
                count: i + 1,
            })?;
            name_to_id.insert(k.id.clone(), id);
            id_to_name.push(k.id.clone());

            let (path, cmd) = match &k.kind {
                KnotKind::SignalOut { path } => {
                    let pid = if let Some(id) = path_index.get(path) {
                        *id
                    } else {
                        let id = HostPathId::try_from(path_names.len()).map_err(|_| {
                            BindError::CapacityExceeded {
                                weave_id: weave_id.clone(),
                                resource: "host path",
                                count: path_names.len() + 1,
                            }
                        })?;
                        path_names.push(path.clone());
                        path_index.insert(path.clone(), id);
                        id
                    };
                    (Some(pid), None)
                }
                KnotKind::EmitCommand { name } => {
                    let cid = if let Some(id) = cmd_index.get(name) {
                        *id
                    } else {
                        let id = CmdId::try_from(cmd_names.len()).map_err(|_| {
                            BindError::CapacityExceeded {
                                weave_id: weave_id.clone(),
                                resource: "command",
                                count: cmd_names.len() + 1,
                            }
                        })?;
                        cmd_names.push(name.clone());
                        cmd_index.insert(name.clone(), id);
                        id
                    };
                    (None, Some(cid))
                }
                _ => (None, None),
            };

            knots.push(ResolvedKnot {
                kind: k.kind.clone(),
                path,
                cmd,
            });
        }

        let mut threads = Vec::new();
        for t in weave.threads() {
            let fk = *name_to_id
                .get(&t.from.knot)
                .ok_or_else(|| BindError::InvalidReference {
                    weave_id: weave_id.clone(),
                    knot: t.from.knot.clone(),
                    port: t.from.port.clone(),
                })?;
            let tk = *name_to_id
                .get(&t.to.knot)
                .ok_or_else(|| BindError::InvalidReference {
                    weave_id: weave_id.clone(),
                    knot: t.to.knot.clone(),
                    port: t.to.port.clone(),
                })?;
            let fs = port_slot(&knots[usize::from(fk)].kind, &t.from.port).ok_or_else(|| {
                BindError::InvalidReference {
                    weave_id: weave_id.clone(),
                    knot: t.from.knot.clone(),
                    port: t.from.port.clone(),
                }
            })?;
            let ts = port_slot(&knots[usize::from(tk)].kind, &t.to.port).ok_or_else(|| {
                BindError::InvalidReference {
                    weave_id: weave_id.clone(),
                    knot: t.to.knot.clone(),
                    port: t.to.port.clone(),
                }
            })?;
            threads.push((fk, fs, tk, ts));
        }

        let topo = topo_order(knots.len(), &threads).ok_or_else(|| BindError::InvalidTopology {
            weave_id: weave_id.clone(),
        })?;

        let n = knots.len();
        let mut inbound_lists: Vec<Vec<(KnotId, PortSlot, PortSlot)>> = vec![Vec::new(); n];
        for &(f, fs, t, ts) in &threads {
            inbound_lists[usize::from(t)].push((f, fs, ts));
        }
        let mut inbound_off = Vec::with_capacity(n + 1);
        let mut inbound_edges = Vec::with_capacity(threads.len());
        inbound_off.push(0);
        for list in &inbound_lists {
            inbound_edges.extend_from_slice(list);
            inbound_off.push(inbound_edges.len() as u32);
        }

        let mut input_slots: Vec<Vec<PortSlot>> = Vec::with_capacity(n);
        let mut clear_port_idx = Vec::new();
        let mut act_signals = 0usize;
        let mut act_emits = 0usize;
        let mut sense_seeds = Vec::new();
        let mut kind_tags: Vec<crate::kind_tag::KindTag> = knots
            .iter()
            .map(|k| crate::kind_tag::KindTag::from_kind(&k.kind))
            .collect();
        for (ki, k) in knots.iter().enumerate() {
            let kid = KnotId::try_from(ki).map_err(|_| BindError::CapacityExceeded {
                weave_id: weave_id.clone(),
                resource: "knot",
                count: ki + 1,
            })?;
            let mut slots = Vec::new();
            for p in ports_of(&k.kind) {
                if p.dir == PortDir::In {
                    slots.push(p.slot);
                    // Only unwired Ins are zeroed each loom; wired Ins are gathered.
                    let wired = inbound_lists[ki].iter().any(|&(_, _, ts)| ts == p.slot);
                    if !wired {
                        clear_port_idx.push(ki * MAX_PORTS + usize::from(p.slot));
                    }
                }
            }
            input_slots.push(slots);
            match &k.kind {
                KnotKind::Constant { value } => {
                    sense_seeds.push(SenseSeed::Constant { kid, value: *value });
                }
                KnotKind::SignalIn => {
                    sense_seeds.push(SenseSeed::SignalIn { kid });
                }
                KnotKind::OnStart => {
                    sense_seeds.push(SenseSeed::OnStart { kid });
                }
                KnotKind::SignalOut { .. } => act_signals += 1,
                KnotKind::EmitCommand { .. } => {
                    act_emits += 1;
                    let enable_wired = inbound_lists[ki]
                        .iter()
                        .any(|&(_, _, ts)| ts == PortSlot::new(1));
                    kind_tags[ki] = crate::kind_tag::KindTag::EmitCommand { enable_wired };
                }
                KnotKind::Random { require_gate } => {
                    let mut min_wired = false;
                    let mut max_wired = false;
                    for &(_, _, ts) in &inbound_lists[ki] {
                        if ts == PortSlot::new(0) {
                            min_wired = true;
                        } else if ts == PortSlot::new(1) {
                            max_wired = true;
                        }
                    }
                    kind_tags[ki] = crate::kind_tag::KindTag::Random {
                        require_gate: *require_gate,
                        min_wired,
                        max_wired,
                    };
                }
                KnotKind::Calc { op: CalcOp::Div } => {
                    if let Some(&(from, _, _)) = inbound_lists[ki]
                        .iter()
                        .find(|&&(_, _, ts)| ts == PortSlot::new(1))
                    {
                        if let KnotKind::Constant { value } = knots[usize::from(from)].kind {
                            kind_tags[ki] =
                                crate::kind_tag::KindTag::CalcDivConst { divisor: value };
                        }
                    }
                }
                _ => {}
            }
        }

        let mut delay_buf = Vec::new();
        let mut delay_off = vec![0u16; n];
        let mut delay_len = vec![0u16; n];
        let delay_head = vec![0u16; n];
        for (i, k) in knots.iter().enumerate() {
            if let KnotKind::Delay { ticks } = k.kind {
                let len = ticks as usize;
                if len > 0 {
                    delay_off[i] = delay_buf.len() as u16;
                    delay_len[i] = ticks;
                    delay_buf.resize(delay_buf.len() + len, ZERO);
                }
            }
        }

        let out_signals = Vec::with_capacity(act_signals);
        let out_emits = Vec::with_capacity(act_emits);

        let base = opts.seed.unwrap_or(Seed(0xC0FF_EE00_D15C_AFEDu64));
        let seed_mix = fnv1a64(weave.id().as_bytes());
        let rng = (base.0 ^ seed_mix) | 1;

        Ok(Runtime {
            knots,
            name_to_id,
            id_to_name,
            path_names,
            cmd_names,
            threads,
            inbound_off,
            inbound_edges,
            clear_port_idx,
            input_slots,
            topo,
            kind_tags,
            sense_seeds,
            sense_values: vec![ZERO; n],
            port_vals: vec![ZERO; n * MAX_PORTS],
            max_ports: MAX_PORTS,
            prev_in: vec![ZERO; n],
            prev_dec: vec![ZERO; n],
            counter: vec![0; n],
            flag: vec![false; n],
            timer_left: vec![0; n],
            on_start_done: vec![false; n],
            delay_buf,
            delay_off,
            delay_len,
            delay_head,
            out_signals,
            out_emits,
            max_emits_per_tick: opts.max_emits_per_tick,
            tick: 0,
            rng,
            seed_mix,
        })
    }

    /// Restore PRNG stream (room retry). Same mix as bind: `seed ^ fnv(weave.id) | 1`.
    pub fn reseed(&mut self, seed: Seed) {
        self.rng = (seed.0 ^ self.seed_mix) | 1;
    }

    /// Next u32 from the bind-seeded xorshift64 stream (`rng` is never zero).
    pub(crate) fn next_rng_u32(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x as u32
    }

    /// Resolve a `SignalIn` author name to a dense sense id (setup only).
    pub fn sense_id(&self, name: &str) -> Option<SenseId> {
        let knot = self.name_to_id.get(name).copied()?;
        if !matches!(self.knots.get(usize::from(knot))?.kind, KnotKind::SignalIn) {
            return None;
        }
        SenseId::try_from(usize::from(knot)).ok()
    }

    /// Resolve a `SignalOut` path string interned at bind.
    pub fn path_id(&self, path: &str) -> Option<HostPathId> {
        self.path_names
            .iter()
            .position(|p| p == path)
            .and_then(|i| HostPathId::try_from(i).ok())
    }

    /// Interned path string for a dense host path id.
    pub fn path_name(&self, id: HostPathId) -> Option<&str> {
        self.path_names.get(usize::from(id)).map(|s| s.as_str())
    }

    /// Interned emit command name for a dense command id.
    pub fn cmd_name(&self, id: CmdId) -> Option<&str> {
        self.cmd_names.get(usize::from(id)).map(|s| s.as_str())
    }

    /// Start a frame: set tick and clear the outbox for this loom.
    pub fn begin_frame(&mut self, time: HostTime) {
        self.tick = time.tick;
        self.out_signals.clear();
        self.out_emits.clear();
    }

    /// Borrow for host sense writes (`set_sense` with dense ids only).
    pub fn port_writer(&mut self) -> PortWriter<'_> {
        PortWriter { rt: self }
    }

    /// Read-only view of SignalOut samples and EmitCommand entries for this frame.
    pub fn outbox(&self) -> Outbox<'_> {
        Outbox {
            signals: &self.out_signals,
            emits: &self.out_emits,
        }
    }

    /// Capacity of the SignalOut outbox buffer (reserved at bind).
    pub fn outbox_signals_capacity(&self) -> usize {
        self.out_signals.capacity()
    }

    /// Length of the flat delay ring (sized at bind).
    pub fn delay_buf_len(&self) -> usize {
        self.delay_buf.len()
    }

    #[inline]
    pub(crate) fn port_index(&self, knot: KnotId, slot: PortSlot) -> usize {
        usize::from(knot) * self.max_ports + usize::from(slot)
    }

    /// Safe OOB-tolerant read (returns ZERO past end). Used by tests and host tooling.
    #[inline]
    pub fn get_port_checked(&self, knot: KnotId, slot: PortSlot) -> Result<Signal, HandleError> {
        let Some(resolved) = self.knots.get(usize::from(knot)) else {
            return Err(HandleError::InvalidKnot { knot });
        };
        if !ports_of(&resolved.kind)
            .iter()
            .any(|info| info.slot == slot)
        {
            return Err(HandleError::InvalidPort { knot, port: slot });
        }
        let i = self.port_index(knot, slot);
        self.port_vals
            .get(i)
            .copied()
            .ok_or(HandleError::InvalidPort { knot, port: slot })
    }

    /// Safe OOB-tolerant write (no-op past end). Used by tests and host tooling.
    #[inline]
    pub fn set_port_checked(
        &mut self,
        knot: KnotId,
        slot: PortSlot,
        v: Signal,
    ) -> Result<(), HandleError> {
        let Some(resolved) = self.knots.get(usize::from(knot)) else {
            return Err(HandleError::InvalidKnot { knot });
        };
        if !ports_of(&resolved.kind)
            .iter()
            .any(|info| info.slot == slot)
        {
            return Err(HandleError::InvalidPort { knot, port: slot });
        }
        let i = self.port_index(knot, slot);
        let p = self
            .port_vals
            .get_mut(i)
            .ok_or(HandleError::InvalidPort { knot, port: slot })?;
        *p = v;
        Ok(())
    }

    /// Alias for checked get (bind-unit tests + OOB safety).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn get_port(&self, knot: KnotId, slot: PortSlot) -> Result<Signal, HandleError> {
        self.get_port_checked(knot, slot)
    }

    /// Alias for checked set (bind-unit tests + OOB safety).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn set_port(
        &mut self,
        knot: KnotId,
        slot: PortSlot,
        v: Signal,
    ) -> Result<(), HandleError> {
        self.set_port_checked(knot, slot, v)
    }

    /// Number of bind-time kind tags (equals knot count after successful bind).
    pub fn kind_tag_count(&self) -> usize {
        self.kind_tags.len()
    }

    /// Flat clear-index count (all In ports across the weave).
    pub fn clear_port_index_count(&self) -> usize {
        self.clear_port_idx.len()
    }

    /// CSR inbound edge count.
    pub fn inbound_edge_count(&self) -> usize {
        self.inbound_edges.len()
    }

    /// Hot-path port read when `knot`/`slot` are bind-validated (in-range).
    #[inline]
    pub(crate) fn get_port_hot(&self, knot: KnotId, slot: PortSlot) -> Signal {
        let i = self.port_index(knot, slot);
        debug_assert!(i < self.port_vals.len());
        self.port_vals[i]
    }

    /// Hot-path port write when `knot`/`slot` are bind-validated.
    #[inline]
    pub(crate) fn set_port_hot(&mut self, knot: KnotId, slot: PortSlot, v: Signal) {
        let i = self.port_index(knot, slot);
        debug_assert!(i < self.port_vals.len());
        self.port_vals[i] = v;
    }

    pub(crate) fn push_signal_out(&mut self, path: HostPathId, value: Signal) {
        self.out_signals.push(SignalOutSample { path, value });
    }

    pub(crate) fn push_emit(&mut self, cmd: CmdId, payload: Signal) {
        if self.out_emits.len() as u16 >= self.max_emits_per_tick {
            return;
        }
        self.out_emits.push(Emit { cmd, payload });
    }
}

fn topo_order(n: usize, threads: &[(KnotId, PortSlot, KnotId, PortSlot)]) -> Option<Vec<KnotId>> {
    let mut indeg = vec![0u32; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(f, _, t, _) in threads {
        let a = usize::from(f);
        let b = usize::from(t);
        if a != b {
            adj[a].push(b);
            indeg[b] += 1;
        }
    }
    let mut q: Vec<usize> = indeg
        .iter()
        .enumerate()
        .filter_map(|(i, d)| if *d == 0 { Some(i) } else { None })
        .collect();
    let mut order = Vec::with_capacity(n);
    while let Some(u) = q.pop() {
        order.push(KnotId::try_from(u).ok()?);
        for &v in &adj[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                q.push(v);
            }
        }
    }
    if order.len() != n {
        return None;
    }
    Some(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wyrd_core::{FlagPriority, KnotKind, ONE};
    use wyrd_graph::Weave;

    #[test]
    fn sense_seeds_lists_only_sense_knots() {
        let mut b = Weave::builder("s").unwrap();
        let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
        let _k_c = b.knot("c", KnotKind::constant(ONE)).unwrap();
        let k_n = b.knot("n", KnotKind::Not).unwrap();
        let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
        let from = b.output(&k_in, "out").unwrap();
        let to = b.input(&k_n, "in").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&k_n, "out").unwrap();
        let to = b.input(&k_out, "in").unwrap();
        b.connect(from, to).unwrap();
        let weave = b.build().unwrap();
        let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
        assert_eq!(rt.sense_seeds.len(), 2, "SignalIn + Constant only");
        assert!(rt
            .sense_seeds
            .iter()
            .any(|s| matches!(s, SenseSeed::SignalIn { .. })));
        assert!(rt
            .sense_seeds
            .iter()
            .any(|s| matches!(s, SenseSeed::Constant { value, .. } if *value == ONE)));
        let mut b = Weave::builder("e").unwrap();
        let k_btn = b.knot("btn", KnotKind::signal_in()).unwrap();
        let k_em = b.knot("em", KnotKind::emit_command("fire")).unwrap();
        let from = b.output(&k_btn, "out").unwrap();
        let to = b.input(&k_em, "trigger").unwrap();
        b.connect(from, to).unwrap();
        let weave = b.build().unwrap();
        let rt = Runtime::bind(
            weave.clone(),
            BindOpts {
                seed: Some(Seed(1)),
                ..BindOpts::default()
            },
        )
        .unwrap();
        let em = *rt.name_to_id.get("em").expect("em knot");
        match rt.kind_tags[usize::from(em)] {
            crate::kind_tag::KindTag::EmitCommand { enable_wired } => {
                assert!(!enable_wired);
            }
            _ => panic!("expected EmitCommand tag"),
        }
    }

    #[test]
    fn cmd_name_and_path_name_lookup() {
        let mut b = Weave::builder("e").unwrap();
        let k_btn = b.knot("btn", KnotKind::signal_in()).unwrap();
        let k_em = b.knot("em", KnotKind::emit_command("fire")).unwrap();
        let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
        let from = b.output(&k_btn, "out").unwrap();
        let to = b.input(&k_em, "trigger").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&k_btn, "out").unwrap();
        let to = b.input(&k_out, "in").unwrap();
        b.connect(from, to).unwrap();
        let weave = b.build().unwrap();
        let rt = Runtime::bind(
            weave.clone(),
            BindOpts {
                seed: Some(Seed(1)),
                ..BindOpts::default()
            },
        )
        .unwrap();
        let cmd = CmdId::try_from(0usize).unwrap();
        assert_eq!(rt.cmd_name(cmd), Some("fire"));
        assert_eq!(rt.cmd_name(CmdId::try_from(99usize).unwrap()), None);
        assert_eq!(
            rt.path_name(HostPathId::try_from(0usize).unwrap()),
            Some("y")
        );
        assert_eq!(rt.path_name(HostPathId::try_from(99usize).unwrap()), None);
    }

    #[test]
    fn topo_order_detects_cycle() {
        let a = KnotId::try_from(0usize).unwrap();
        let b = KnotId::try_from(1usize).unwrap();
        let threads = [
            (a, PortSlot::new(1), b, PortSlot::new(0)),
            (b, PortSlot::new(1), a, PortSlot::new(0)),
        ];
        assert_eq!(topo_order(2, &threads), None);
    }

    #[test]
    fn checked_port_access_reports_oob_without_mutation() {
        let mut b = Weave::builder("x").unwrap();
        let _k_c = b.knot("c", KnotKind::constant(ONE)).unwrap();
        let weave = b.build().unwrap();
        let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
        let far = KnotId::try_from(999usize).unwrap();
        assert_eq!(
            rt.get_port(far, PortSlot::new(0)),
            Err(HandleError::InvalidKnot { knot: far })
        );
        assert_eq!(
            rt.set_port(far, PortSlot::new(0), ONE),
            Err(HandleError::InvalidKnot { knot: far })
        );
        let _ = FlagPriority::SetWins;
    }

    #[test]
    fn clear_only_unwired_ins_and_div_const_specializes() {
        use wyrd_core::CalcOp;
        let mut b = Weave::builder("fl").unwrap();
        let k_f = b
            .knot("f", KnotKind::flag(FlagPriority::SetWins, false))
            .unwrap();
        let k_o = b.knot("o", KnotKind::signal_out("y")).unwrap();
        let from = b.output(&k_f, "out").unwrap();
        let to = b.input(&k_o, "in").unwrap();
        b.connect(from, to).unwrap();
        let weave = b.build().unwrap();
        let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
        assert_eq!(
            rt.clear_port_index_count(),
            3,
            "unwired Flag Ins must clear"
        );

        let mut b = Weave::builder("dv").unwrap();
        let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
        let k_one = b.knot("one", KnotKind::constant(ONE)).unwrap();
        let k_d = b.knot("d", KnotKind::Calc { op: CalcOp::Div }).unwrap();
        let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
        let from = b.output(&k_in, "out").unwrap();
        let to = b.input(&k_d, "a").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&k_one, "out").unwrap();
        let to = b.input(&k_d, "b").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&k_d, "out").unwrap();
        let to = b.input(&k_out, "in").unwrap();
        b.connect(from, to).unwrap();
        let weave = b.build().unwrap();
        let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
        let d = *rt.name_to_id.get("d").expect("div knot");
        match rt.kind_tags[usize::from(d)] {
            crate::kind_tag::KindTag::CalcDivConst { divisor } => {
                assert_eq!(divisor, ONE);
            }
            other => panic!("expected CalcDivConst, got {other:?}"),
        }
    }

    /// Bind builds KindTag cache, flat clear indices, and CSR inbound.
    #[test]
    fn bind_builds_hot_path_tables() {
        let mut b = Weave::builder("h").unwrap();
        let k_a = b.knot("a", KnotKind::signal_in()).unwrap();
        let k_n = b.knot("n", KnotKind::not()).unwrap();
        let k_o = b.knot("o", KnotKind::signal_out("y")).unwrap();
        let from = b.output(&k_a, "out").unwrap();
        let to = b.input(&k_n, "in").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&k_n, "out").unwrap();
        let to = b.input(&k_o, "in").unwrap();
        b.connect(from, to).unwrap();
        let weave = b.build().unwrap();
        let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
        assert_eq!(rt.kind_tag_count(), weave.knots().len());
        assert_eq!(rt.clear_port_index_count(), 0);
        assert_eq!(rt.inbound_edge_count(), 2);
        assert_eq!(rt.inbound_off.len(), weave.knots().len() + 1);
        let n_id = KnotId::try_from(1usize).unwrap();
        rt.set_port_hot(n_id, PortSlot::new(0), ONE);
        assert_eq!(rt.get_port_hot(n_id, PortSlot::new(0)), ONE);
        assert_eq!(rt.get_port_checked(n_id, PortSlot::new(0)), Ok(ONE));
    }
}
