use std::collections::BTreeMap;
use std::string::String;
use std::vec;
use std::vec::Vec;

use wyrd_core::{
    port_slot, CmdId, HostPathId, HostTime, KnotId, KnotKind, PortSlot, Result, Seed, Signal,
    WyrdError, ZERO,
};
use wyrd_graph::{validate, Budget, Weave};

use crate::outbox::{Emit, Outbox, PortWriter, SignalOutSample};

#[derive(Clone, Debug, Default)]
pub struct BindOpts {
    pub seed: Option<Seed>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedKnot {
    pub(crate) kind: KnotKind,
    /// For SignalOut / Emit after intern
    pub(crate) path: Option<HostPathId>,
    pub(crate) cmd: Option<CmdId>,
}

/// Bound runtime: dense buffers, topo order, intern tables.
pub struct Runtime {
    pub(crate) knots: Vec<ResolvedKnot>,
    /// Author name → KnotId
    name_to_id: BTreeMap<String, KnotId>,
    /// KnotId → author name
    id_to_name: Vec<String>,
    path_names: Vec<String>,
    cmd_names: Vec<String>,
    /// Threads: (from_knot, from_slot, to_knot, to_slot)
    pub(crate) threads: Vec<(KnotId, PortSlot, KnotId, PortSlot)>,
    pub(crate) topo: Vec<KnotId>,
    /// Host-fed sense outputs (SignalIn).
    pub(crate) sense_values: Vec<Signal>,
    /// Port value store: indexed by (knot_idx * MAX_PORTS + slot)
    port_vals: Vec<Signal>,
    max_ports: usize,
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
    tick: u64,
    #[allow(dead_code)]
    seed: Option<Seed>,
}

const MAX_PORTS: usize = 8;

impl Runtime {
    pub fn bind(weave: &Weave, opts: BindOpts) -> Result<Self> {
        validate(weave, &Budget::default())?;

        let mut name_to_id = BTreeMap::new();
        let mut id_to_name = Vec::new();
        let mut path_names = Vec::new();
        let mut cmd_names = Vec::new();
        let mut path_index: BTreeMap<String, HostPathId> = BTreeMap::new();
        let mut cmd_index: BTreeMap<String, CmdId> = BTreeMap::new();

        let mut knots = Vec::with_capacity(weave.knots.len());
        for (i, k) in weave.knots.iter().enumerate() {
            let id = KnotId(i as u16);
            name_to_id.insert(k.id.clone(), id);
            id_to_name.push(k.id.clone());

            let (path, cmd) = match &k.kind {
                KnotKind::SignalOut { path } => {
                    let pid = *path_index.entry(path.clone()).or_insert_with(|| {
                        let id = HostPathId(path_names.len() as u16);
                        path_names.push(path.clone());
                        id
                    });
                    (Some(pid), None)
                }
                KnotKind::EmitCommand { name } => {
                    let cid = *cmd_index.entry(name.clone()).or_insert_with(|| {
                        let id = CmdId(cmd_names.len() as u16);
                        cmd_names.push(name.clone());
                        id
                    });
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
        for t in &weave.threads {
            let fk = *name_to_id.get(&t.from.knot).ok_or(WyrdError::UnknownKnot)?;
            let tk = *name_to_id.get(&t.to.knot).ok_or(WyrdError::UnknownKnot)?;
            let fs = port_slot(&knots[fk.0 as usize].kind, &t.from.port)
                .ok_or(WyrdError::UnknownPort)?;
            let ts =
                port_slot(&knots[tk.0 as usize].kind, &t.to.port).ok_or(WyrdError::UnknownPort)?;
            threads.push((fk, fs, tk, ts));
        }

        let topo = topo_order(knots.len(), &threads)?;

        let n = knots.len();
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

        Ok(Runtime {
            knots,
            name_to_id,
            id_to_name,
            path_names,
            cmd_names,
            threads,
            topo,
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
            out_signals: Vec::new(),
            out_emits: Vec::new(),
            tick: 0,
            seed: opts.seed,
        })
    }

    pub fn sense_id(&self, name: &str) -> Option<KnotId> {
        self.name_to_id.get(name).copied()
    }

    pub fn path_id(&self, path: &str) -> Option<HostPathId> {
        self.path_names
            .iter()
            .position(|p| p == path)
            .map(|i| HostPathId(i as u16))
    }

    pub fn path_name(&self, id: HostPathId) -> &str {
        self.path_names
            .get(id.0 as usize)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn cmd_name(&self, id: CmdId) -> &str {
        self.cmd_names
            .get(id.0 as usize)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn begin_frame(&mut self, time: HostTime) {
        self.tick = time.tick;
        self.out_signals.clear();
        self.out_emits.clear();
        // clear non-sense inputs for this frame — refilled from threads during loom
    }

    pub fn port_writer(&mut self) -> PortWriter<'_> {
        PortWriter { rt: self }
    }

    pub fn outbox(&self) -> Outbox<'_> {
        Outbox {
            signals: &self.out_signals,
            emits: &self.out_emits,
        }
    }

    pub(crate) fn get_port(&self, knot: KnotId, slot: PortSlot) -> Signal {
        let i = knot.0 as usize * self.max_ports + slot.0 as usize;
        self.port_vals.get(i).copied().unwrap_or(ZERO)
    }

    pub(crate) fn set_port(&mut self, knot: KnotId, slot: PortSlot, v: Signal) {
        let i = knot.0 as usize * self.max_ports + slot.0 as usize;
        if let Some(p) = self.port_vals.get_mut(i) {
            *p = v;
        }
    }

    pub(crate) fn push_signal_out(&mut self, path: HostPathId, value: Signal) {
        self.out_signals.push(SignalOutSample { path, value });
    }

    pub(crate) fn push_emit(&mut self, cmd: CmdId, payload: Signal) {
        self.out_emits.push(Emit { cmd, payload });
    }
}

fn topo_order(
    n: usize,
    threads: &[(KnotId, PortSlot, KnotId, PortSlot)],
) -> Result<Vec<KnotId>> {
    let mut indeg = vec![0u32; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(f, _, t, _) in threads {
        let a = f.0 as usize;
        let b = t.0 as usize;
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
        order.push(KnotId(u as u16));
        for &v in &adj[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                q.push(v);
            }
        }
    }
    if order.len() != n {
        return Err(WyrdError::Cycle);
    }
    Ok(order)
}
