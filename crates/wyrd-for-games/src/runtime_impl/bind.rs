//! Bind: turn a validated [`Weave`] into a dense, executable [`Runtime`].
//!
//! Interns host paths and command names, builds CSR inbound edges, topo order,
//! kind-dispatch tags, delay rings, and sense seed lists so [`Runtime::loom`]
//! allocates no topology after bind. Host resolves `sense_id` / `path_id` once
//! at setup; the hot path uses only dense ids.

#![allow(clippy::result_large_err)] // Preserve contextual public BindError payloads.

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::authoring::{validate, Budget, Weave};
use crate::foundation::{
    port_slot, ports_of, CalcOp, HostTime, KnotId, KnotKind, PortDir, PortSlot, Seed, Signal, ZERO,
};

use crate::runtime_impl::error::{BindError, HandleError};
use crate::runtime_impl::handles::{CmdId, HostPathId, KnotHandle, SenseId};
use crate::runtime_impl::outbox::{Emit, Outbox, PortWriter, SignalOutSample};

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
    /// Hard cap on `EmitCommand` outbox entries per frame (default 8).
    ///
    /// Further emits in the same frame are dropped without panicking. Their
    /// exact count is exposed through [`Outbox::dropped_emits`] until the next
    /// [`Runtime::begin_frame`]. A cap of zero drops every emit.
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
    pub(crate) owner: usize,
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
    pub(crate) kind_tags: Vec<crate::runtime_impl::kind_tag::KindTag>,
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
    dropped_emits: usize,
    max_emits_per_tick: u16,
    tick: u64,
    /// Deterministic xorshift state for Random knots (never zero).
    pub(crate) rng: u64,
    /// `fnv1a64(weave.id)` mixed into seeds at bind and [`Self::reseed`].
    seed_mix: u64,
}

const MAX_PORTS: usize = 8;
static NEXT_RUNTIME_OWNER: AtomicUsize = AtomicUsize::new(1);

fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Convert an authoring-order index to the compact runtime knot id.
///
/// Both bind passes use this guard: the first protects name and intern tables,
/// and the second protects the dense hot-path arrays built from those tables.
fn dense_knot_id(index: usize, weave_id: &str) -> Result<KnotId, BindError> {
    KnotId::try_from(index).map_err(|_| BindError::CapacityExceeded {
        weave_id: String::from(weave_id),
        resource: "knot",
        count: index + 1,
    })
}

/// Intern a SignalOut path while preserving compact host-path ids.
fn intern_host_path(
    owner: usize,
    path: &str,
    path_index: &mut BTreeMap<String, HostPathId>,
    path_names: &mut Vec<String>,
    weave_id: &str,
) -> Result<HostPathId, BindError> {
    if let Some(id) = path_index.get(path) {
        return Ok(*id);
    }

    let index = u16::try_from(path_names.len()).map_err(|_| BindError::CapacityExceeded {
        weave_id: String::from(weave_id),
        resource: "host path",
        count: path_names.len() + 1,
    })?;
    let id = HostPathId::new(owner, index);
    path_names.push(String::from(path));
    path_index.insert(String::from(path), id);
    Ok(id)
}

/// Intern an EmitCommand name while preserving compact command ids.
fn intern_command(
    owner: usize,
    name: &str,
    cmd_index: &mut BTreeMap<String, CmdId>,
    cmd_names: &mut Vec<String>,
    weave_id: &str,
) -> Result<CmdId, BindError> {
    if let Some(id) = cmd_index.get(name) {
        return Ok(*id);
    }

    let index = u16::try_from(cmd_names.len()).map_err(|_| BindError::CapacityExceeded {
        weave_id: String::from(weave_id),
        resource: "command",
        count: cmd_names.len() + 1,
    })?;
    let id = CmdId::new(owner, index);
    cmd_names.push(String::from(name));
    cmd_index.insert(String::from(name), id);
    Ok(id)
}

/// Add a delay extent while preserving the error reported by the original bind
/// phase when the host architecture's usize capacity would overflow.
fn checked_delay_buffer_len(
    current_len: usize,
    len: usize,
    weave_id: &str,
) -> Result<usize, BindError> {
    current_len
        .checked_add(len)
        .ok_or_else(|| BindError::CapacityExceeded {
            weave_id: String::from(weave_id),
            resource: "delay buffer",
            count: usize::MAX,
        })
}

/// Calculate the next delay-ring extent before allocating the backing buffer.
///
/// `current_len` is passed separately so the compact-index guard remains
/// directly testable even though a validated weave cannot reach its impossible
/// overflow state.
fn delay_buffer_layout(
    current_len: usize,
    len: usize,
    weave_id: &str,
) -> Result<(u16, usize), BindError> {
    let offset = u16::try_from(current_len).map_err(|_| BindError::CapacityExceeded {
        weave_id: String::from(weave_id),
        resource: "delay buffer offset",
        count: current_len,
    })?;
    let new_len = checked_delay_buffer_len(current_len, len, weave_id)?;
    if new_len > u16::MAX as usize {
        return Err(BindError::CapacityExceeded {
            weave_id: String::from(weave_id),
            resource: "delay buffer",
            count: new_len,
        });
    }
    Ok((offset, new_len))
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

        Self::bind_validated(weave, opts, weave_id)
    }

    /// Build dense state from a weave that already passed structural and budget validation.
    ///
    /// Keeping this phase separate makes the defensive error handling below
    /// testable without exposing a way to bypass validation to callers.
    fn bind_validated(weave: Weave, opts: BindOpts, weave_id: String) -> Result<Self, BindError> {
        Self::bind_validated_with_preinterned_names(weave, opts, weave_id, Vec::new(), Vec::new())
    }

    /// Internal bind entrypoint with explicit preinterned names for capacity
    /// validation. Normal binding always starts with empty tables.
    fn bind_validated_with_preinterned_names(
        weave: Weave,
        opts: BindOpts,
        weave_id: String,
        mut path_names: Vec<String>,
        mut cmd_names: Vec<String>,
    ) -> Result<Self, BindError> {
        let owner = NEXT_RUNTIME_OWNER.fetch_add(1, Ordering::Relaxed);
        reserve_owner(owner, &weave_id)?;

        let mut name_to_id = BTreeMap::new();
        let mut id_to_name = Vec::new();
        let mut path_index: BTreeMap<String, HostPathId> = BTreeMap::new();
        let mut cmd_index: BTreeMap<String, CmdId> = BTreeMap::new();

        let mut knots = Vec::with_capacity(weave.knots().len());
        for (i, k) in weave.knots().iter().enumerate() {
            let id = dense_knot_id(i, &weave_id)?;
            name_to_id.insert(k.id.clone(), id);
            id_to_name.push(k.id.clone());

            let (path, cmd) = match &k.kind {
                KnotKind::SignalOut { path, .. } => (
                    Some(intern_host_path(
                        owner,
                        path,
                        &mut path_index,
                        &mut path_names,
                        &weave_id,
                    )?),
                    None,
                ),
                KnotKind::EmitCommand { name } => (
                    None,
                    Some(intern_command(
                        owner,
                        name,
                        &mut cmd_index,
                        &mut cmd_names,
                        &weave_id,
                    )?),
                ),
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
        let mut inbound_lists: Vec<Vec<(KnotId, PortSlot, PortSlot)>> = alloc::vec![Vec::new(); n];
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
        let mut kind_tags: Vec<crate::runtime_impl::kind_tag::KindTag> = knots
            .iter()
            .map(|k| crate::runtime_impl::kind_tag::KindTag::from_kind(&k.kind))
            .collect();
        for (ki, k) in knots.iter().enumerate() {
            let kid = dense_knot_id(ki, &weave_id)?;
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
                KnotKind::Constant { value, .. } => {
                    sense_seeds.push(SenseSeed::Constant { kid, value: *value });
                }
                KnotKind::SignalIn { .. } => {
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
                    kind_tags[ki] =
                        crate::runtime_impl::kind_tag::KindTag::EmitCommand { enable_wired };
                }
                KnotKind::Random { .. } => {
                    let mut min_wired = false;
                    let mut max_wired = false;
                    for &(_, _, ts) in &inbound_lists[ki] {
                        if ts == PortSlot::new(0) {
                            min_wired = true;
                        } else if ts == PortSlot::new(1) {
                            max_wired = true;
                        }
                    }
                    kind_tags[ki] = kind_tags[ki].with_random_wiring(min_wired, max_wired);
                }
                KnotKind::Calc {
                    domain,
                    op: CalcOp::Div,
                } => {
                    if let Some(&(from, _, _)) = inbound_lists[ki]
                        .iter()
                        .find(|&&(_, _, ts)| ts == PortSlot::new(1))
                    {
                        if let KnotKind::Constant { value, .. } = knots[usize::from(from)].kind {
                            kind_tags[ki] = crate::runtime_impl::kind_tag::KindTag::calc_div_const(
                                *domain, value,
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        let mut delay_buf = Vec::new();
        let mut delay_off = alloc::vec![0u16; n];
        let mut delay_len = alloc::vec![0u16; n];
        let delay_head = alloc::vec![0u16; n];
        for (i, k) in knots.iter().enumerate() {
            if let KnotKind::Delay { ticks } = k.kind {
                let len = ticks as usize;
                if len > 0 {
                    let (offset, new_len) = delay_buffer_layout(delay_buf.len(), len, &weave_id)?;
                    delay_off[i] = offset;
                    delay_len[i] = ticks;
                    delay_buf.resize(new_len, ZERO);
                }
            }
        }

        let out_signals = Vec::with_capacity(act_signals);
        let out_emits = Vec::with_capacity(act_emits);

        let base = opts.seed.unwrap_or(Seed(0xC0FF_EE00_D15C_AFEDu64));
        let seed_mix = fnv1a64(weave.id().as_bytes());
        let rng = (base.0 ^ seed_mix) | 1;

        Ok(Runtime {
            owner,
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
            sense_values: alloc::vec![ZERO; n],
            port_vals: alloc::vec![ZERO; n * MAX_PORTS],
            max_ports: MAX_PORTS,
            prev_in: alloc::vec![ZERO; n],
            prev_dec: alloc::vec![ZERO; n],
            counter: alloc::vec![0; n],
            flag: alloc::vec![false; n],
            timer_left: alloc::vec![0; n],
            on_start_done: alloc::vec![false; n],
            delay_buf,
            delay_off,
            delay_len,
            delay_head,
            out_signals,
            out_emits,
            dropped_emits: 0,
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
        if !matches!(
            self.knots.get(usize::from(knot))?.kind,
            KnotKind::SignalIn { .. }
        ) {
            return None;
        }
        Some(SenseId::new(self.owner, knot.get()))
    }

    /// Resolve an author knot name for checked tooling access.
    pub fn knot_id(&self, name: &str) -> Option<KnotHandle> {
        self.name_to_id
            .get(name)
            .map(|knot| KnotHandle::new(self.owner, knot.get()))
    }

    /// Resolve a `SignalOut` path string interned at bind.
    pub fn path_id(&self, path: &str) -> Option<HostPathId> {
        self.path_names
            .iter()
            .position(|p| p == path)
            .and_then(|i| u16::try_from(i).ok())
            .map(|index| HostPathId::new(self.owner, index))
    }

    /// Resolve an `EmitCommand` name interned at bind.
    pub fn cmd_id(&self, name: &str) -> Option<CmdId> {
        self.cmd_names
            .iter()
            .position(|candidate| candidate == name)
            .and_then(|i| u16::try_from(i).ok())
            .map(|index| CmdId::new(self.owner, index))
    }

    /// Interned path string for a dense host path id.
    pub fn path_name(&self, id: HostPathId) -> Result<&str, HandleError> {
        self.ensure_owner(id.owner, "host path")?;
        self.path_names
            .get(usize::from(id.index))
            .map(|s| s.as_str())
            .ok_or(HandleError::InvalidHostPath { path: id })
    }

    /// Interned emit command name for a dense command id.
    pub fn cmd_name(&self, id: CmdId) -> Result<&str, HandleError> {
        self.ensure_owner(id.owner, "command")?;
        self.cmd_names
            .get(usize::from(id.index))
            .map(|s| s.as_str())
            .ok_or(HandleError::InvalidCommand { cmd: id })
    }

    /// Start a frame: set tick and clear acts and dropped-emit telemetry.
    pub fn begin_frame(&mut self, time: HostTime) {
        self.tick = time.tick;
        self.out_signals.clear();
        self.out_emits.clear();
        self.dropped_emits = 0;
    }

    /// Borrow for host sense writes (`set_sense` with dense ids only).
    pub fn port_writer(&mut self) -> PortWriter<'_> {
        PortWriter { rt: self }
    }

    /// Read-only view of acts and dropped-emit telemetry for this frame.
    pub fn outbox(&self) -> Outbox<'_> {
        Outbox {
            signals: &self.out_signals,
            emits: &self.out_emits,
            dropped_emits: self.dropped_emits,
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
    pub fn get_port_checked(
        &self,
        knot: KnotHandle,
        slot: PortSlot,
    ) -> Result<Signal, HandleError> {
        self.ensure_owner(knot.owner, "knot")?;
        let dense = KnotId::try_from(usize::from(knot.index))
            .map_err(|_| HandleError::InvalidKnot { knot })?;
        let Some(resolved) = self.knots.get(usize::from(dense)) else {
            return Err(HandleError::InvalidKnot { knot });
        };
        if !ports_of(&resolved.kind)
            .iter()
            .any(|info| info.slot == slot)
        {
            return Err(HandleError::InvalidPort { knot, port: slot });
        }
        let i = self.port_index(dense, slot);
        self.port_vals
            .get(i)
            .copied()
            .ok_or(HandleError::InvalidPort { knot, port: slot })
    }

    /// Safe OOB-tolerant write (no-op past end). Used by tests and host tooling.
    #[inline]
    pub fn set_port_checked(
        &mut self,
        knot: KnotHandle,
        slot: PortSlot,
        v: Signal,
    ) -> Result<(), HandleError> {
        self.ensure_owner(knot.owner, "knot")?;
        let dense = KnotId::try_from(usize::from(knot.index))
            .map_err(|_| HandleError::InvalidKnot { knot })?;
        let Some(resolved) = self.knots.get(usize::from(dense)) else {
            return Err(HandleError::InvalidKnot { knot });
        };
        if !ports_of(&resolved.kind)
            .iter()
            .any(|info| info.slot == slot)
        {
            return Err(HandleError::InvalidPort { knot, port: slot });
        }
        let i = self.port_index(dense, slot);
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
        let handle = KnotHandle::new(self.owner, knot.get());
        self.get_port_checked(handle, slot)
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
        let handle = KnotHandle::new(self.owner, knot.get());
        self.set_port_checked(handle, slot, v)
    }

    fn ensure_owner(&self, owner: usize, handle: &'static str) -> Result<(), HandleError> {
        if owner == self.owner {
            Ok(())
        } else {
            Err(HandleError::ForeignRuntime { handle })
        }
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
        if self.out_emits.len() >= usize::from(self.max_emits_per_tick) {
            self.dropped_emits = self.dropped_emits.saturating_add(1);
            return;
        }
        self.out_emits.push(Emit { cmd, payload });
    }
}

fn reserve_owner(owner: usize, weave_id: &str) -> Result<(), BindError> {
    if owner == usize::MAX {
        return Err(BindError::CapacityExceeded {
            weave_id: String::from(weave_id),
            resource: "runtime owner token",
            count: owner,
        });
    }
    Ok(())
}

fn topo_order(n: usize, threads: &[(KnotId, PortSlot, KnotId, PortSlot)]) -> Option<Vec<KnotId>> {
    let mut indeg = alloc::vec![0u32; n];
    let mut adj: Vec<Vec<usize>> = alloc::vec![Vec::new(); n];
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
    use crate::authoring::{KnotDef, PortRefDef, ThreadDef, Weave, WeaveDef};
    use crate::foundation::{CalcOp, FlagPriority, KnotKind, NumericPath, SignalDomain, ONE};
    use std::vec;

    fn unchecked_weave(id: &str, knots: Vec<KnotDef>, threads: Vec<ThreadDef>) -> Weave {
        Weave::from_validated(WeaveDef {
            id: String::from(id),
            numeric: NumericPath::compiled(),
            knots,
            threads,
        })
    }

    fn bind_unchecked(id: &str, knots: Vec<KnotDef>, threads: Vec<ThreadDef>) -> BindError {
        Runtime::bind_validated(
            unchecked_weave(id, knots, threads),
            BindOpts::default(),
            String::from(id),
        )
        .err()
        .expect("malformed internal weave must not bind")
    }

    fn knot(id: &str, kind: KnotKind) -> KnotDef {
        KnotDef {
            id: String::from(id),
            kind,
        }
    }

    #[test]
    fn sense_seeds_lists_only_sense_knots() {
        let mut b = Weave::builder("s").unwrap();
        let k_in = b
            .knot("in", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap();
        let _k_c = b
            .knot("c", KnotKind::constant(ONE, SignalDomain::Bool))
            .unwrap();
        let k_n = b.knot("n", KnotKind::Not).unwrap();
        let k_out = b
            .knot("out", KnotKind::signal_out("y", SignalDomain::Bool))
            .unwrap();
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
        let k_btn = b
            .knot("btn", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap();
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
        assert!(matches!(
            rt.kind_tags[usize::from(em)],
            crate::runtime_impl::kind_tag::KindTag::EmitCommand {
                enable_wired: false
            }
        ));
    }

    #[test]
    fn cmd_name_and_path_name_lookup() {
        let mut b = Weave::builder("e").unwrap();
        let k_btn = b
            .knot("btn", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap();
        let k_em = b.knot("em", KnotKind::emit_command("fire")).unwrap();
        let k_out = b
            .knot("out", KnotKind::signal_out("y", SignalDomain::Bool))
            .unwrap();
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
        let cmd = rt.knots[usize::from(*rt.name_to_id.get("em").unwrap())]
            .cmd
            .unwrap();
        assert_eq!(rt.cmd_name(cmd), Ok("fire"));
        let invalid_cmd = CmdId::new(rt.owner, 99);
        assert_eq!(
            rt.cmd_name(invalid_cmd),
            Err(HandleError::InvalidCommand { cmd: invalid_cmd })
        );
        let path = rt.path_id("y").unwrap();
        assert_eq!(rt.path_name(path), Ok("y"));
        let invalid_path = HostPathId::new(rt.owner, 99);
        assert_eq!(
            rt.path_name(invalid_path),
            Err(HandleError::InvalidHostPath { path: invalid_path })
        );
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
        let _k_c = b
            .knot("c", KnotKind::constant(ONE, SignalDomain::Bool))
            .unwrap();
        let weave = b.build().unwrap();
        let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
        let far = KnotId::try_from(999usize).unwrap();
        let far_handle = KnotHandle::new(rt.owner, far.get());
        assert_eq!(
            rt.get_port(far, PortSlot::new(0)),
            Err(HandleError::InvalidKnot { knot: far_handle })
        );
        assert_eq!(
            rt.set_port(far, PortSlot::new(0), ONE),
            Err(HandleError::InvalidKnot { knot: far_handle })
        );
        let _ = FlagPriority::SetWins;
    }

    #[test]
    fn dropped_emit_count_saturates() {
        let mut b = Weave::builder("emit-saturation").unwrap();
        let input = b
            .knot("input", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap();
        let emit = b.knot("emit", KnotKind::emit_command("fire")).unwrap();
        let from = b.output(&input, "out").unwrap();
        let to = b.input(&emit, "trigger").unwrap();
        b.connect(from, to).unwrap();
        let mut rt = Runtime::bind(
            b.build().unwrap(),
            BindOpts {
                max_emits_per_tick: 0,
                ..BindOpts::default()
            },
        )
        .unwrap();
        let cmd = rt.cmd_id("fire").unwrap();
        rt.dropped_emits = usize::MAX;

        rt.push_emit(cmd, ONE);

        assert_eq!(rt.dropped_emits, usize::MAX);
        assert!(rt.out_emits.is_empty());
    }

    #[test]
    fn clear_only_unwired_ins_and_div_const_specializes() {
        use crate::foundation::CalcOp;
        let mut b = Weave::builder("fl").unwrap();
        let k_f = b
            .knot("f", KnotKind::flag(FlagPriority::SetWins, false))
            .unwrap();
        let k_o = b
            .knot("o", KnotKind::signal_out("y", SignalDomain::Bool))
            .unwrap();
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
        let k_in = b
            .knot("in", KnotKind::signal_in(SignalDomain::Level))
            .unwrap();
        let k_one = b
            .knot("one", KnotKind::constant(ONE, SignalDomain::Level))
            .unwrap();
        let k_d = b
            .knot(
                "d",
                KnotKind::Calc {
                    domain: SignalDomain::Level,
                    op: CalcOp::Div,
                },
            )
            .unwrap();
        let k_out = b
            .knot("out", KnotKind::signal_out("y", SignalDomain::Level))
            .unwrap();
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
        assert!(matches!(
            rt.kind_tags[usize::from(d)],
            crate::runtime_impl::kind_tag::KindTag::CalcDivLevelConst { divisor } if divisor == ONE
        ));
    }

    /// Bind builds KindTag cache, flat clear indices, and CSR inbound.
    #[test]
    fn bind_builds_hot_path_tables() {
        let mut b = Weave::builder("h").unwrap();
        let k_a = b
            .knot("a", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap();
        let k_n = b.knot("n", KnotKind::not()).unwrap();
        let k_o = b
            .knot("o", KnotKind::signal_out("y", SignalDomain::Bool))
            .unwrap();
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
        let n_handle = rt.knot_id("n").unwrap();
        assert_eq!(rt.get_port_checked(n_handle, PortSlot::new(0)), Ok(ONE));
    }

    #[test]
    fn defensive_bind_phase_rejects_invalid_internal_definitions() {
        let constant = || knot("constant", KnotKind::constant(ONE, SignalDomain::Bool));
        let out = || knot("out", KnotKind::signal_out("out", SignalDomain::Bool));

        let error = bind_unchecked(
            "missing-from",
            vec![constant()],
            vec![ThreadDef {
                from: PortRefDef::new("missing", "out"),
                to: PortRefDef::new("constant", "out"),
            }],
        );
        assert!(matches!(
            error,
            BindError::InvalidReference { knot, port, .. } if knot == "missing" && port == "out"
        ));

        let error = bind_unchecked(
            "missing-to",
            vec![constant()],
            vec![ThreadDef {
                from: PortRefDef::new("constant", "out"),
                to: PortRefDef::new("missing", "in"),
            }],
        );
        assert!(matches!(
            error,
            BindError::InvalidReference { knot, port, .. } if knot == "missing" && port == "in"
        ));

        let error = bind_unchecked(
            "missing-from-port",
            vec![constant(), out()],
            vec![ThreadDef {
                from: PortRefDef::new("constant", "in"),
                to: PortRefDef::new("out", "in"),
            }],
        );
        assert!(matches!(
            error,
            BindError::InvalidReference { knot, port, .. } if knot == "constant" && port == "in"
        ));

        let error = bind_unchecked(
            "missing-to-port",
            vec![constant(), out()],
            vec![ThreadDef {
                from: PortRefDef::new("constant", "out"),
                to: PortRefDef::new("out", "out"),
            }],
        );
        assert!(matches!(
            error,
            BindError::InvalidReference { knot, port, .. } if knot == "out" && port == "out"
        ));

        let error = bind_unchecked(
            "cycle",
            vec![knot("a", KnotKind::Not), knot("b", KnotKind::Not)],
            vec![
                ThreadDef {
                    from: PortRefDef::new("a", "out"),
                    to: PortRefDef::new("b", "in"),
                },
                ThreadDef {
                    from: PortRefDef::new("b", "out"),
                    to: PortRefDef::new("a", "in"),
                },
            ],
        );
        assert!(matches!(error, BindError::InvalidTopology { .. }));
    }

    #[test]
    fn defensive_bind_phase_checks_dense_capacities_before_allocation() {
        assert_eq!(
            reserve_owner(usize::MAX, "owner"),
            Err(BindError::CapacityExceeded {
                weave_id: String::from("owner"),
                resource: "runtime owner token",
                count: usize::MAX,
            })
        );
        assert_eq!(reserve_owner(1, "owner"), Ok(()));

        let excessive_knots = (0..=(usize::from(u16::MAX) + 1))
            .map(|_| knot("", KnotKind::OnStart))
            .collect();
        let error = bind_unchecked("too-many-knots", excessive_knots, Vec::new());
        assert_eq!(
            error,
            BindError::CapacityExceeded {
                weave_id: String::from("too-many-knots"),
                resource: "knot",
                count: usize::from(u16::MAX) + 2,
            }
        );

        let delay_knots = (0..256)
            .map(|_| knot("", KnotKind::Delay { ticks: 256 }))
            .collect();
        let error = bind_unchecked("delay-capacity", delay_knots, Vec::new());
        assert_eq!(
            error,
            BindError::CapacityExceeded {
                weave_id: String::from("delay-capacity"),
                resource: "delay buffer",
                count: usize::from(u16::MAX) + 1,
            }
        );
    }

    #[test]
    fn defensive_compact_capacity_guards_report_the_next_entry() {
        let overflow_index = usize::from(u16::MAX) + 1;
        let mut path_index = BTreeMap::new();
        let mut path_names = vec![String::new(); overflow_index];
        assert_eq!(
            intern_host_path(
                1,
                "overflow",
                &mut path_index,
                &mut path_names,
                "path-capacity",
            ),
            Err(BindError::CapacityExceeded {
                weave_id: String::from("path-capacity"),
                resource: "host path",
                count: overflow_index + 1,
            })
        );

        let mut cmd_index = BTreeMap::new();
        let mut cmd_names = vec![String::new(); overflow_index];
        assert_eq!(
            intern_command(
                1,
                "overflow",
                &mut cmd_index,
                &mut cmd_names,
                "command-capacity",
            ),
            Err(BindError::CapacityExceeded {
                weave_id: String::from("command-capacity"),
                resource: "command",
                count: overflow_index + 1,
            })
        );

        assert_eq!(
            delay_buffer_layout(overflow_index, 1, "delay-offset-capacity"),
            Err(BindError::CapacityExceeded {
                weave_id: String::from("delay-offset-capacity"),
                resource: "delay buffer offset",
                count: overflow_index,
            })
        );
        assert_eq!(
            checked_delay_buffer_len(usize::MAX, 1, "delay-overflow"),
            Err(BindError::CapacityExceeded {
                weave_id: String::from("delay-overflow"),
                resource: "delay buffer",
                count: usize::MAX,
            })
        );
    }

    #[test]
    fn preinterned_tables_propagate_path_and_command_capacity_errors() {
        let full_names = vec![String::new(); usize::from(u16::MAX) + 1];
        let path_error = Runtime::bind_validated_with_preinterned_names(
            unchecked_weave(
                "path-capacity",
                vec![knot(
                    "output",
                    KnotKind::signal_out("too-many-paths", SignalDomain::Bool),
                )],
                Vec::new(),
            ),
            BindOpts::default(),
            String::from("path-capacity"),
            full_names,
            Vec::new(),
        )
        .err()
        .expect("the next host path must exceed its compact id space");
        assert!(matches!(
            path_error,
            BindError::CapacityExceeded {
                resource: "host path",
                count,
                ..
            } if count == usize::from(u16::MAX) + 2
        ));

        let full_names = vec![String::new(); usize::from(u16::MAX) + 1];
        let command_error = Runtime::bind_validated_with_preinterned_names(
            unchecked_weave(
                "command-capacity",
                vec![knot("emit", KnotKind::emit_command("too-many-commands"))],
                Vec::new(),
            ),
            BindOpts::default(),
            String::from("command-capacity"),
            Vec::new(),
            full_names,
        )
        .err()
        .expect("the next command must exceed its compact id space");
        assert!(matches!(
            command_error,
            BindError::CapacityExceeded {
                resource: "command",
                count,
                ..
            } if count == usize::from(u16::MAX) + 2
        ));
    }

    #[test]
    fn div_with_a_dynamic_rhs_keeps_the_general_dispatch_tag() {
        let mut b = Weave::builder("dynamic-divisor").unwrap();
        let lhs = b
            .knot("lhs", KnotKind::signal_in(SignalDomain::Level))
            .unwrap();
        let rhs = b
            .knot("rhs", KnotKind::signal_in(SignalDomain::Level))
            .unwrap();
        let div = b
            .knot(
                "div",
                KnotKind::Calc {
                    domain: SignalDomain::Level,
                    op: CalcOp::Div,
                },
            )
            .unwrap();
        b.connect(b.output(&lhs, "out").unwrap(), b.input(&div, "a").unwrap())
            .unwrap();
        b.connect(b.output(&rhs, "out").unwrap(), b.input(&div, "b").unwrap())
            .unwrap();

        let rt = Runtime::bind(b.build().unwrap(), BindOpts::default()).unwrap();
        let div = rt.name_to_id["div"];
        assert!(matches!(
            rt.kind_tags[usize::from(div)],
            crate::runtime_impl::kind_tag::KindTag::CalcDivLevel
        ));
    }

    #[test]
    fn div_without_an_rhs_edge_keeps_the_general_dispatch_tag_defensively() {
        let weave = unchecked_weave(
            "unwired-divisor",
            vec![knot(
                "div",
                KnotKind::calc(CalcOp::Div, SignalDomain::Level),
            )],
            Vec::new(),
        );
        let runtime =
            Runtime::bind_validated(weave, BindOpts::default(), String::from("unwired-divisor"))
                .expect("the defensive bind phase can represent an unwired calc");

        assert!(matches!(
            runtime.kind_tags[0],
            crate::runtime_impl::kind_tag::KindTag::CalcDivLevel
        ));
    }

    #[test]
    fn sense_id_rejects_existing_non_sense_knots() {
        let mut b = Weave::builder("sense-id-kind").unwrap();
        b.knot("constant", KnotKind::constant(ONE, SignalDomain::Bool))
            .unwrap();
        let rt = Runtime::bind(b.build().unwrap(), BindOpts::default()).unwrap();

        assert_eq!(rt.sense_id("constant"), None);
    }
}
