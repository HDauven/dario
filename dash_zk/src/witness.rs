//! Witness extraction: turns a finished [`ZkSim`](crate::ZkSim) run into the
//! flat data the circom circuit consumes.
//!
//! For every scheduled obstacle we compute its player-overlap window
//! `[w1, w2]` (contiguous because obstacle x is strictly decreasing) and a
//! clearance interval `[a, b]` that the circuit must prove collision-free:
//!
//! - `Cleared`:      b = min(w2, T)
//! - `Killed(th)`:   b = min(w2, T, th - 1)  (removed by fireball at th)
//! - `Damaged(td)`:  b = td - 1              (collision happened at td)
//! - `InvulnTouch(ti)`: b = ti - 1           (destroyed harmlessly at ti)
//!
//! If `b < a` the obstacle needs no clearance proof (status/exempt). Ground
//! obstacles additionally get the index of the jump whose airborne interval
//! covers `[a, b]` (player grounded anywhere in the window of a ground
//! obstacle means collision, so a covering jump must exist).

use crate::*;

/// Sentinel tick used for padding/absent values (> MAX_TICKS).
pub const T_NONE: u32 = MAX_TICKS + 400;

#[derive(Clone, Copy, Default, Debug)]
pub struct ObsWitness {
    /// 0 cleared, 1 killed, 2 damaged, 3 invuln touch, 4 not-reached/padding.
    pub status: u32,
    /// Overlap window (x-overlap with player), T_NONE if never reached.
    pub w1: u32,
    pub w2: u32,
    /// Clearance interval [a, b]; b < a means nothing to prove.
    pub a: u32,
    pub b: u32,
    /// Ground obstacles: index of the covering jump (T_NONE-like u32::MAX
    /// when no clearance needed).
    pub jump_idx: u32,
    /// Event tick for killed/damaged/touched, else T_NONE.
    pub event_tick: u32,
}

/// Merged form-event timeline entry.
/// kind: 0 espresso, 1 chili, 2 cape, 3 damage, 4 invuln-touch (no-op).
#[derive(Clone, Copy, Default, Debug)]
pub struct FormEv {
    pub tick: u32,
    pub kind: u32,
    /// Form value after applying this event (0 Regular, 1 Super, 2 Fire,
    /// 3 Cape, 4 GameOver).
    pub form_after: u32,
}

pub const MAX_FORM_EVENTS: usize = MAX_PICKUPS + MAX_DAMAGES;

#[derive(Clone)]
pub struct RunWitness {
    pub seed: u64,
    pub score: u64,
    pub ticks: u32,
    pub ground: [ObsWitness; MAX_GROUND],
    pub bats: [ObsWitness; MAX_BATS],
    pub form_events: [FormEv; MAX_FORM_EVENTS],
    pub form_event_count: usize,
    pub jumps: [JumpEv; MAX_JUMPS],
    pub jump_count: usize,
    pub pickups: [PickupEv; MAX_PICKUPS],
    pub pickup_count: usize,
    pub kills: [KillEv; MAX_KILLS],
    pub kill_count: usize,
}

/// First tick `t >= spawn` where the obstacle's hitbox x-overlaps the
/// player, and the last such tick. Returns None if the window starts after
/// `t_max`.
fn overlap_window(
    spawn: u32,
    left_off100: i64,
    right_off100: i64,
    t_max: u32,
) -> Option<(u32, u32)> {
    // x(t) = OBS_X0_100 - (d100(t) - d100(spawn-1)); strictly decreasing.
    // w1: first t with x + left_off < PLAYER_RIGHT100
    // w2: last  t with x + right_off > PLAYER_LEFT100
    let x_at = |t: u32| world_x100(OBS_X0_100, spawn, t);
    let mut w1 = None;
    let mut t = spawn;
    while t <= t_max {
        if x_at(t) + left_off100 < PLAYER_RIGHT100 {
            w1 = Some(t);
            break;
        }
        t += 1;
    }
    let w1 = w1?;
    let mut w2 = w1;
    let mut t = w1;
    loop {
        if x_at(t) + right_off100 > PLAYER_LEFT100 {
            w2 = t;
            t += 1;
        } else {
            break;
        }
    }
    Some((w1, w2))
}

impl RunWitness {
    /// Extracts the witness from a finished run.
    pub fn extract(sim: &ZkSim) -> Self {
        let sched = sim.schedule();
        let t_end = sim.ticks();

        let mut ground = [ObsWitness::default(); MAX_GROUND];
        for i in 0..sched.ground_count {
            let g = sched.ground[i];
            let win = overlap_window(g.spawn_tick, 4 * FP100, (g.w as i64 - 4) * FP100, t_end);
            ground[i] = obs_witness(win, sim.ground_status()[i], t_end);
        }
        for i in sched.ground_count..MAX_GROUND {
            ground[i].status = 4;
            ground[i].w1 = T_NONE;
            ground[i].w2 = T_NONE;
            ground[i].a = T_NONE;
            ground[i].b = 0;
            ground[i].event_tick = T_NONE;
            ground[i].jump_idx = u32::MAX;
        }

        let mut bats = [ObsWitness::default(); MAX_BATS];
        for i in 0..sched.bat_count {
            let b = sched.bats[i];
            let win = overlap_window(b.spawn_tick, 0, (BAT_W as i64) * FP100, t_end);
            bats[i] = obs_witness(win, sim.bat_status()[i], t_end);
        }
        for i in sched.bat_count..MAX_BATS {
            bats[i].status = 4;
            bats[i].w1 = T_NONE;
            bats[i].w2 = T_NONE;
            bats[i].a = T_NONE;
            bats[i].b = 0;
            bats[i].event_tick = T_NONE;
            bats[i].jump_idx = u32::MAX;
        }

        // Covering jump for ground obstacles that need clearance: the jump
        // whose airborne ticks [tick, tick + n_land - 1] contain [a, b].
        for i in 0..sched.ground_count {
            let ow = &mut ground[i];
            if ow.b < ow.a {
                ow.jump_idx = u32::MAX;
                continue;
            }
            let mut found = u32::MAX;
            for j in 0..sim.jump_count {
                let jp = sim.jumps[j];
                if jp.tick <= ow.a && ow.b <= jp.tick + jp.n_land - 1 {
                    found = j as u32;
                    break;
                }
            }
            ow.jump_idx = found;
        }

        // Merged form-event timeline (pickups + damages, by tick; sim
        // processes pickups before damage within a tick).
        let mut form_events = [FormEv::default(); MAX_FORM_EVENTS];
        let mut n = 0usize;
        let mut pi = 0usize;
        let mut di = 0usize;
        let mut form = dario_fsm::DarioState::Regular;
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
            let (tick, kind) = if pt <= dt {
                let it = sched.items[sim.pickups[pi].item_idx as usize];
                pi += 1;
                (pt, it.kind as u32)
            } else {
                let d = sim.damages[di];
                di += 1;
                (dt, if d.invuln_touch { 4 } else { 3 })
            };
            let ev = match kind {
                0 => Some(dario_fsm::Event::Espresso),
                1 => Some(dario_fsm::Event::ChiliPepper),
                2 => Some(dario_fsm::Event::TableClothCape),
                3 => Some(dario_fsm::Event::TakeDamage),
                _ => None,
            };
            if let Some(ev) = ev {
                form = dario_fsm::transition(form, ev);
            }
            if n < MAX_FORM_EVENTS {
                form_events[n] = FormEv {
                    tick,
                    kind,
                    form_after: form as u32,
                };
                n += 1;
            }
        }

        RunWitness {
            seed: sim.seed,
            score: sim.score(),
            ticks: t_end,
            ground,
            bats,
            form_events,
            form_event_count: n,
            jumps: sim.jumps,
            jump_count: sim.jump_count,
            pickups: sim.pickups,
            pickup_count: sim.pickup_count,
            kills: sim.kills,
            kill_count: sim.kill_count,
        }
    }
}

fn obs_witness(win: Option<(u32, u32)>, status: ObsStatus, t_end: u32) -> ObsWitness {
    let (w1, w2) = match win {
        Some(w) => w,
        None => {
            return ObsWitness {
                status: 4,
                w1: T_NONE,
                w2: T_NONE,
                a: T_NONE,
                b: 0,
                jump_idx: u32::MAX,
                event_tick: T_NONE,
            }
        }
    };
    let (code, b, ev) = match status {
        ObsStatus::Cleared => (0, w2.min(t_end), T_NONE),
        ObsStatus::Killed(th) => (1, w2.min(t_end).min(th.saturating_sub(1)), th),
        ObsStatus::Damaged(td) => (2, td.saturating_sub(1), td),
        ObsStatus::InvulnTouch(ti) => (3, ti.saturating_sub(1), ti),
    };
    ObsWitness {
        status: code,
        w1,
        w2,
        a: w1,
        b,
        jump_idx: u32::MAX,
        event_tick: ev,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Autopilot that plays reasonably: jump over ground obstacles.
    fn autopilot_run(seed: u64) -> ZkSim {
        let mut sim = ZkSim::new(seed);
        let mut snap = [0i32; 40 * 6];
        let mut prev = 0u8;
        while !sim.over() && sim.ticks() < MAX_TICKS {
            let n = sim.snapshot(&mut snap);
            let mut want = false;
            for rec in snap[..n].chunks(6) {
                if rec[0] == 0 && rec[1] != KIND_BAT {
                    let dist = rec[2] - (PLAYER_X + PLAYER_W);
                    if (0..=60).contains(&dist) {
                        want = true;
                    }
                }
            }
            let input = if want && prev & INPUT_JUMP == 0 {
                INPUT_JUMP
            } else {
                0
            };
            prev = input;
            sim.tick(input);
        }
        sim
    }

    #[test]
    fn windows_bracket_actual_overlap() {
        let sim = autopilot_run(42);
        let sched = sim.schedule();
        let w = RunWitness::extract(&sim);
        for i in 0..sched.ground_count {
            let g = sched.ground[i];
            let ow = w.ground[i];
            if ow.status == 4 {
                continue;
            }
            // Boundary conditions the circuit will check.
            let x = |t: u32| world_x100(OBS_X0_100, g.spawn_tick, t);
            let left = |t: u32| x(t) + 4 * FP100;
            let right = |t: u32| x(t) + (g.w as i64 - 4) * FP100;
            assert!(left(ow.w1) < PLAYER_RIGHT100);
            if ow.w1 > g.spawn_tick {
                assert!(left(ow.w1 - 1) >= PLAYER_RIGHT100);
            }
            assert!(right(ow.w2) > PLAYER_LEFT100);
            assert!(right(ow.w2 + 1) <= PLAYER_LEFT100);
        }
    }

    #[test]
    fn cleared_ground_obstacles_have_covering_jump() {
        let sim = autopilot_run(42);
        assert!(
            sim.ticks() > 400,
            "autopilot died too early: {}",
            sim.ticks()
        );
        let w = RunWitness::extract(&sim);
        let sched = sim.schedule();
        for i in 0..sched.ground_count {
            let ow = w.ground[i];
            if ow.status == 0 && ow.b >= ow.a {
                assert_ne!(
                    ow.jump_idx,
                    u32::MAX,
                    "cleared ground obstacle {i} [{},{}] lacks covering jump",
                    ow.a,
                    ow.b
                );
                let jp = w.jumps[ow.jump_idx as usize];
                // Endpoint clearance (what the circuit checks) must hold.
                for &t in &[ow.a, ow.b] {
                    let n = (t - jp.tick).max(1);
                    let y = GROUND_Y * FP + jump_disp(jp.v0, jp.cape, n) as i32;
                    let top = (GROUND_Y - sched.ground[i].h + 4) * FP;
                    assert!(
                        y - PLAYER_BOX_BOT_FP <= top,
                        "obstacle {i} endpoint {t} not clear: y={y} top={top}"
                    );
                }
            }
        }
    }

    #[test]
    fn form_timeline_is_sorted_and_ends_consistently() {
        let sim = autopilot_run(1234);
        let w = RunWitness::extract(&sim);
        for k in 1..w.form_event_count {
            assert!(w.form_events[k].tick >= w.form_events[k - 1].tick);
        }
        if sim.over() && sim.ticks() < MAX_TICKS {
            // Death run: last event is a fatal damage at T.
            let last = w.form_events[w.form_event_count - 1];
            assert_eq!(last.kind, 3);
            assert_eq!(last.form_after, dario_fsm::DarioState::GameOver as u32);
            assert_eq!(last.tick, sim.ticks());
        }
    }

    #[test]
    fn bat_windows_are_short() {
        let sim = autopilot_run(42);
        let w = RunWitness::extract(&sim);
        for i in 0..sim.schedule().bat_count {
            let ow = w.bats[i];
            if ow.status != 4 && ow.b >= ow.a {
                assert!(
                    ow.b - ow.a + 1 <= 8,
                    "bat window too long: {}",
                    ow.b - ow.a + 1
                );
            }
        }
    }
}
