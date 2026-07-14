//! Builds the snarkjs `input.json` for the dash_zk circom circuit from a
//! finished [`ZkSim`] run. Shared by the native exporter binary and the
//! browser wasm wrapper, so the witness layout is identical everywhere.

use crate::witness::{RunWitness, T_NONE};
use crate::*;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Write as _;

const NG: usize = MAX_GROUND; // 128
const NB: usize = MAX_BATS; // 24
const NI: usize = MAX_SCHED_ITEMS; // 56
const NJ: usize = MAX_JUMPS; // 160
const NE: usize = MAX_FORM_EVENTS; // 64
const NK: usize = MAX_KILLS; // 32
const TPAD: u32 = 4000;

struct Entry {
    tick: u32,
    kind: u32, // 0..2 pickup, 3 damage, 4 touch (padding: 4, inactive)
    act: u32,
    isel: Option<usize>,
    osel: Option<usize>, // global obstacle index (ground i, bat NG+j)
}

fn build_entries(sim: &ZkSim) -> Vec<Entry> {
    let sched = sim.schedule();
    let mut out = Vec::with_capacity(NE);
    let (mut pi, mut di) = (0usize, 0usize);
    while pi < sim.pickup_count || di < sim.damage_count {
        let pt = if pi < sim.pickup_count {
            sim.pickups[pi].tick
        } else {
            u32::MAX
        };
        let dt = if di < sim.damage_count {
            sim.damages[di].tick
        } else {
            u32::MAX
        };
        if pt <= dt {
            let p = sim.pickups[pi];
            let it = sched.items[p.item_idx as usize];
            out.push(Entry {
                tick: pt,
                kind: it.kind as u32,
                act: 1,
                isel: Some(p.item_idx as usize),
                osel: None,
            });
            pi += 1;
        } else {
            let d = sim.damages[di];
            let gidx = if d.class == 0 {
                d.idx as usize
            } else {
                NG + d.idx as usize
            };
            out.push(Entry {
                tick: dt,
                kind: if d.invuln_touch { 4 } else { 3 },
                act: 1,
                isel: None,
                osel: Some(gidx),
            });
            di += 1;
        }
    }
    assert!(out.len() <= NE, "too many form events");
    while out.len() < NE {
        out.push(Entry {
            tick: TPAD,
            kind: 4,
            act: 0,
            isel: None,
            osel: None,
        });
    }
    out
}

/// Virtual-list index of the last jump with tick <= t (0 = none/virtual).
fn last_jump_vidx(sim: &ZkSim, t: u32) -> usize {
    let mut idx = 0usize;
    for j in 0..sim.jump_count {
        if sim.jumps[j].tick <= t {
            idx = j + 1;
        } else {
            break;
        }
    }
    idx
}

/// Virtual-list index for FormAt: number of active entries with tick < t.
fn form_vidx(entries: &[Entry], t: u32) -> usize {
    entries.iter().filter(|e| e.act == 1 && e.tick < t).count()
}

fn tri_qr(p: u32) -> (u32, u32) {
    (p / BAT_PERIOD, p % BAT_PERIOD)
}

struct J(String);

impl J {
    fn new() -> Self {
        J("{\n".to_string())
    }
    fn num(&mut self, name: &str, v: i64) {
        let _ = writeln!(self.0, "\"{}\": \"{}\",", name, v);
    }
    fn arr(&mut self, name: &str, vals: &[i64]) {
        let _ = write!(self.0, "\"{}\": [", name);
        for (i, v) in vals.iter().enumerate() {
            if i > 0 {
                self.0.push(',');
            }
            let _ = write!(self.0, "\"{}\"", v);
        }
        self.0.push_str("],\n");
    }
    fn arr_str(&mut self, name: &str, vals: &[String]) {
        let _ = write!(self.0, "\"{}\": [", name);
        for (i, v) in vals.iter().enumerate() {
            if i > 0 {
                self.0.push(',');
            }
            let _ = write!(self.0, "\"{}\"", v);
        }
        self.0.push_str("],\n");
    }
    fn arr2(&mut self, name: &str, vals: &[Vec<i64>]) {
        let _ = writeln!(self.0, "\"{}\": [", name);
        for (i, row) in vals.iter().enumerate() {
            if i > 0 {
                self.0.push_str(",\n");
            }
            self.0.push('[');
            for (k, v) in row.iter().enumerate() {
                if k > 0 {
                    self.0.push(',');
                }
                let _ = write!(self.0, "\"{}\"", v);
            }
            self.0.push(']');
        }
        self.0.push_str("\n],\n");
    }
    fn finish(mut self) -> String {
        // strip trailing ",\n"
        self.0.truncate(self.0.len() - 2);
        self.0.push_str("\n}\n");
        self.0
    }
}

fn onehot(n: usize, idx: Option<usize>) -> Vec<i64> {
    let mut v = vec![0i64; n];
    if let Some(i) = idx {
        v[i] = 1;
    }
    v
}

/// Serializes the full circuit input (publics + private witness) for a
/// finished run. `acct` is the caller's 96-byte account as 6 little-endian
/// u128 limbs; use zeros when no account is bound yet.
pub fn build_input_json(sim: &ZkSim, acct: &[u128; 6]) -> String {
    let sched = sim.schedule();
    let w = RunWitness::extract(sim);
    let entries = build_entries(sim);
    let ticks = sim.ticks();

    let mut j = J::new();

    // ---- publics ----
    j.num("score", sim.score() as i64);
    j.num("ticks", i64::from(ticks));
    j.num("groundCount", sched.ground_count as i64);
    j.num("batCount", sched.bat_count as i64);
    j.num("itemCount", sched.item_count as i64);
    j.arr(
        "gspawn",
        &(0..NG)
            .map(|i| i64::from(sched.ground[i].spawn_tick))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "gw",
        &(0..NG)
            .map(|i| i64::from(sched.ground[i].w))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "gh",
        &(0..NG)
            .map(|i| i64::from(sched.ground[i].h))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "bspawn",
        &(0..NB)
            .map(|i| i64::from(sched.bats[i].spawn_tick))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "bbase",
        &(0..NB)
            .map(|i| i64::from(sched.bats[i].base_y_px))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "bphase",
        &(0..NB)
            .map(|i| i64::from(sched.bats[i].phase0))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "ispawn",
        &(0..NI)
            .map(|i| i64::from(sched.items[i].spawn_tick))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "ikind",
        &(0..NI)
            .map(|i| i64::from(sched.items[i].kind))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "iy",
        &(0..NI)
            .map(|i| i64::from(sched.items[i].y_px))
            .collect::<Vec<_>>(),
    );
    j.arr_str(
        "acct",
        &acct.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
    );

    // ---- jumps ----
    let jn = sim.jump_count;
    j.arr(
        "jtick",
        &(0..NJ)
            .map(|i| {
                if i < jn {
                    i64::from(sim.jumps[i].tick)
                } else {
                    i64::from(TPAD)
                }
            })
            .collect::<Vec<_>>(),
    );
    j.arr(
        "jact",
        &(0..NJ).map(|i| i64::from(i < jn)).collect::<Vec<_>>(),
    );
    let jfsel: Vec<Vec<i64>> = (0..NJ)
        .map(|i| {
            let t = if i < jn { sim.jumps[i].tick } else { TPAD };
            onehot(NE + 1, Some(form_vidx(&entries, t)))
        })
        .collect();
    j.arr2("jfsel", &jfsel);

    // ---- timeline entries ----
    j.arr(
        "etick",
        &entries
            .iter()
            .map(|e| i64::from(e.tick))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "ekind",
        &entries
            .iter()
            .map(|e| i64::from(e.kind))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "eact",
        &entries.iter().map(|e| i64::from(e.act)).collect::<Vec<_>>(),
    );
    j.arr2(
        "eisel",
        &entries
            .iter()
            .map(|e| onehot(NI, e.isel))
            .collect::<Vec<_>>(),
    );
    j.arr2(
        "eosel",
        &entries
            .iter()
            .map(|e| onehot(NG + NB, e.osel))
            .collect::<Vec<_>>(),
    );
    j.arr2(
        "ejsel",
        &entries
            .iter()
            .map(|e| {
                if e.act == 1 {
                    onehot(NJ + 1, Some(last_jump_vidx(sim, e.tick)))
                } else {
                    onehot(NJ + 1, None)
                }
            })
            .collect::<Vec<_>>(),
    );
    let mut etq = vec![0i64; NE];
    let mut etr = vec![0i64; NE];
    for (i, e) in entries.iter().enumerate() {
        if let Some(g) = e.osel {
            if g >= NG {
                let b = sched.bats[g - NG];
                let (q, r) = tri_qr(b.phase0 + e.tick - b.spawn_tick);
                etq[i] = i64::from(q);
                etr[i] = i64::from(r);
            }
        }
    }
    j.arr("etq", &etq);
    j.arr("etr", &etr);

    // ---- kills (sorted by fire tick) ----
    let mut kills: Vec<KillEv> = (0..sim.kill_count).map(|i| sim.kills[i]).collect();
    kills.sort_unstable_by_key(|k| k.fire_tick);
    let kn = kills.len();
    j.arr(
        "kfire",
        &(0..NK)
            .map(|i| {
                if i < kn {
                    i64::from(kills[i].fire_tick)
                } else {
                    i64::from(TPAD)
                }
            })
            .collect::<Vec<_>>(),
    );
    j.arr(
        "khit",
        &(0..NK)
            .map(|i| {
                if i < kn {
                    i64::from(kills[i].hit_tick)
                } else {
                    i64::from(TPAD)
                }
            })
            .collect::<Vec<_>>(),
    );
    j.arr(
        "kact",
        &(0..NK).map(|i| i64::from(i < kn)).collect::<Vec<_>>(),
    );
    j.arr2(
        "kosel",
        &(0..NK)
            .map(|i| {
                if i < kn {
                    let k = kills[i];
                    let g = if k.target_class == 0 {
                        k.target_idx as usize
                    } else {
                        NG + k.target_idx as usize
                    };
                    onehot(NG + NB, Some(g))
                } else {
                    onehot(NG + NB, None)
                }
            })
            .collect::<Vec<_>>(),
    );
    j.arr2(
        "kfsel",
        &(0..NK)
            .map(|i| {
                if i < kn {
                    onehot(NE + 1, Some(form_vidx(&entries, kills[i].fire_tick)))
                } else {
                    onehot(NE + 1, None)
                }
            })
            .collect::<Vec<_>>(),
    );
    j.arr2(
        "kjsel",
        &(0..NK)
            .map(|i| {
                if i < kn {
                    onehot(NJ + 1, Some(last_jump_vidx(sim, kills[i].fire_tick)))
                } else {
                    onehot(NJ + 1, None)
                }
            })
            .collect::<Vec<_>>(),
    );
    let mut ktq = vec![0i64; NK];
    let mut ktr = vec![0i64; NK];
    for i in 0..kn {
        let k = kills[i];
        if k.target_class == 1 {
            let b = sched.bats[k.target_idx as usize];
            let (q, r) = tri_qr(b.phase0 + k.hit_tick - b.spawn_tick);
            ktq[i] = i64::from(q);
            ktr[i] = i64::from(r);
        }
    }
    j.arr("ktq", &ktq);
    j.arr("ktr", &ktr);

    // ---- ground obstacles ----
    let wt = |t: u32| -> i64 {
        if t >= T_NONE {
            i64::from(TPAD)
        } else {
            i64::from(t)
        }
    };
    j.arr(
        "gw1",
        &(0..NG).map(|i| wt(w.ground[i].w1)).collect::<Vec<_>>(),
    );
    j.arr(
        "gw2",
        &(0..NG).map(|i| wt(w.ground[i].w2)).collect::<Vec<_>>(),
    );
    j.arr2(
        "gs",
        &(0..NG)
            .map(|i| onehot(5, Some(w.ground[i].status as usize)))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "gevt",
        &(0..NG)
            .map(|i| wt(w.ground[i].event_tick))
            .collect::<Vec<_>>(),
    );
    j.arr2(
        "gcsel",
        &(0..NG)
            .map(|i| {
                let ow = &w.ground[i];
                let nc = ow.status != 4 && ow.a <= ow.b;
                if nc {
                    assert!(
                        ow.jump_idx != u32::MAX,
                        "ground obstacle lacks covering jump"
                    );
                    onehot(NJ, Some(ow.jump_idx as usize))
                } else {
                    onehot(NJ, None)
                }
            })
            .collect::<Vec<_>>(),
    );

    // ---- bats ----
    j.arr(
        "bw1",
        &(0..NB).map(|i| wt(w.bats[i].w1)).collect::<Vec<_>>(),
    );
    j.arr(
        "bw2",
        &(0..NB).map(|i| wt(w.bats[i].w2)).collect::<Vec<_>>(),
    );
    j.arr2(
        "bs",
        &(0..NB)
            .map(|i| onehot(5, Some(w.bats[i].status as usize)))
            .collect::<Vec<_>>(),
    );
    j.arr(
        "bevt",
        &(0..NB)
            .map(|i| wt(w.bats[i].event_tick))
            .collect::<Vec<_>>(),
    );
    let bnc: Vec<bool> = (0..NB)
        .map(|i| w.bats[i].status != 4 && w.bats[i].a <= w.bats[i].b)
        .collect();
    j.arr2(
        "bjsel",
        &(0..NB)
            .map(|i| {
                if bnc[i] {
                    onehot(NJ + 1, Some(last_jump_vidx(sim, w.bats[i].w1)))
                } else {
                    onehot(NJ + 1, None)
                }
            })
            .collect::<Vec<_>>(),
    );
    let mut btq = vec![vec![0i64; 8]; NB];
    let mut btr = vec![vec![0i64; 8]; NB];
    for i in 0..NB {
        if !bnc[i] {
            continue;
        }
        let ow = &w.bats[i];
        let b = sched.bats[i];
        for d in 0..8u32 {
            let tau = ow.w1 + d;
            if tau <= ow.b {
                let (q, r) = tri_qr(b.phase0 + tau - b.spawn_tick);
                btq[i][d as usize] = i64::from(q);
                btr[i][d as usize] = i64::from(r);
            }
        }
    }
    j.arr2("btq", &btq);
    j.arr2("btr", &btr);

    // ---- score division ----
    let dt = d100(ticks);
    let denom = i64::from(FP) * 100 * 50; // 1_280_000
    j.num("scoreQ", dt / denom);
    j.num("scoreR", dt % denom);

    j.finish()
}
