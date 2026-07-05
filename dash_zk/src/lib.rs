//! # Dario Dash ZK — browser-provable game core
//!
//! A 30 Hz, integer-only variant of Dario Dash whose *entire* physics is
//! expressible in closed form, so a Groth16 circuit (circom/snarkjs, proven
//! in-browser) can verify a run from a short list of *events* instead of
//! replaying every tick.
//!
//! Design (see session spec `zk2-game-spec.md`):
//! - All horizontal positions/distances use **fp100** units
//!   (1 px = 25_600 fp100) so scroll speed needs no per-tick floor division:
//!   `s100(t) = min(281_600 + 256·t, 648_600)` and the total distance
//!   `D100(t)` has an exact piecewise-quadratic closed form.
//! - Vertical positions use **fp** units (1 px = 256 fp). A jump fixes its
//!   parameters (initial velocity, cape-glide flag) at the press tick; the
//!   trajectory is then a discrete parabola with an optional linear glide
//!   tail — exactly evaluable at any tick.
//! - The obstacle/item schedule is a pure function of the seed (xorshift64*)
//!   computed *natively* (browser wasm + on-chain contract). It is bound to
//!   the proof as packed public inputs; nothing pseudo-random happens inside
//!   the circuit.
//! - The sim records the events (jumps, pickups, fireball kills, damage,
//!   invulnerable touches) that constitute the ZK witness.
#![no_std]

extern crate alloc;

use dario_fsm::{transition, DarioState, Event};

pub mod input_json;
pub mod witness;

pub const TICK_HZ: u32 = 30;
/// Vertical fixed point: 1 px = 256 fp.
pub const FP: i32 = 256;
/// Horizontal fixed point: 1 px = 25_600 fp100.
pub const FP100: i64 = 25_600;

pub const WORLD_W: i32 = 960;
pub const WORLD_H: i32 = 540;
pub const GROUND_Y: i32 = 464;

pub const INPUT_JUMP: u8 = 1;
pub const INPUT_FIRE: u8 = 2;
pub const INPUT_MASK: u8 = INPUT_JUMP | INPUT_FIRE;

/// Ranked runs are capped at 2 minutes.
pub const MAX_TICKS: u32 = 2 * 60 * TICK_HZ; // 3600

// --- Physics constants (30 Hz) ---
/// Gravity, fp/tick² (≈ 2600 px/s²).
pub const GRAVITY: i32 = 740;
/// Jump velocity, fp/tick (≈ -920 px/s).
pub const JUMP_V: i32 = -7850;
/// Super-form jump velocity, fp/tick (= -1080 px/s).
pub const SUPER_JUMP_V: i32 = -9216;
/// Cape auto-glide: falling speed cap, fp/tick (= 150 px/s).
pub const GLIDE_CAP: i32 = 1280;

/// Scroll speed, fp100/tick: `min(BASE + ACCEL·t, MAX)`.
pub const SPEED_BASE100: i64 = 281_600; // 330 px/s
pub const SPEED_ACCEL100: i64 = 256;
pub const SPEED_MAX100: i64 = 648_600; // ~760 px/s
/// Last tick before the speed cap engages: BASE + ACCEL·t < MAX.
pub const SPEED_CAP_TICK: i64 = 1433;

pub const FIREBALL_SPEED100: i64 = 529_000; // ~620 px/s
pub const FIREBALL_COOLDOWN: u32 = 14;
pub const FIREBALL_SIZE: i32 = 18;
/// Fireball spawn x (player right edge), fp100.
pub const FIREBALL_X0_100: i64 = (PLAYER_X + PLAYER_W) as i64 * FP100;
/// Ticks a fireball stays in flight before leaving the screen.
pub const FIREBALL_LIFE: u32 = 39;

pub const INVULN_TICKS: u32 = 38;

pub const PLAYER_X: i32 = 130;
pub const PLAYER_W: i32 = 46;
pub const PLAYER_H: i32 = 74;
/// Player hitbox in fp100 (x) and fp offsets (y).
pub const PLAYER_LEFT100: i64 = (PLAYER_X + 8) as i64 * FP100;
pub const PLAYER_RIGHT100: i64 = (PLAYER_X + PLAYER_W - 8) as i64 * FP100;
/// Hitbox top = player_y - 68 px; bottom = player_y - 4 px.
pub const PLAYER_BOX_TOP_FP: i32 = (PLAYER_H - 6) * FP;
pub const PLAYER_BOX_BOT_FP: i32 = 4 * FP;

/// Obstacle spawn x (left edge), fp100.
pub const OBS_X0_100: i64 = (WORLD_W + 40) as i64 * FP100;
/// Item spawn x, fp100.
pub const ITEM_X0_100: i64 = (WORLD_W + 30) as i64 * FP100;
pub const ITEM_SIZE: i32 = 34;

pub const KIND_BARREL: i32 = 0;
pub const KIND_PIPE: i32 = 1;
pub const KIND_BAT: i32 = 2;

pub const ITEM_ESPRESSO: i32 = 0;
pub const ITEM_CHILI: i32 = 1;
pub const ITEM_CAPE: i32 = 2;

/// Bat hover: triangle wave, period 36 ticks, step 280 fp per unit
/// (peak amplitude ±18·280 fp ≈ ±19.7 px). No divisions.
pub const BAT_PERIOD: u32 = 36;
pub const BAT_STEP: i32 = 280;
pub const BAT_W: i32 = 40;
pub const BAT_H: i32 = 32;

// --- Schedule / witness capacity (mirrored by the circuit) ---
pub const MAX_GROUND: usize = 128;
pub const MAX_BATS: usize = 24;
pub const MAX_SCHED_ITEMS: usize = 56;
pub const MAX_JUMPS: usize = 160;
pub const MAX_PICKUPS: usize = 56;
pub const MAX_KILLS: usize = 32;
pub const MAX_DAMAGES: usize = 8;

/// Total scrolled distance after `t` ticks, fp100. Exact closed form of
/// `Σ_{u=1..t} min(BASE + ACCEL·u, MAX)`.
pub fn d100(t: u32) -> i64 {
    let t = i64::from(t);
    if t <= SPEED_CAP_TICK {
        SPEED_BASE100 * t + (SPEED_ACCEL100 / 2) * t * (t + 1)
    } else {
        let base = SPEED_BASE100 * SPEED_CAP_TICK
            + (SPEED_ACCEL100 / 2) * SPEED_CAP_TICK * (SPEED_CAP_TICK + 1);
        base + SPEED_MAX100 * (t - SPEED_CAP_TICK)
    }
}

/// Scroll speed at tick `t`, fp100/tick.
pub fn s100(t: u32) -> i64 {
    let s = SPEED_BASE100 + SPEED_ACCEL100 * i64::from(t);
    if s > SPEED_MAX100 {
        SPEED_MAX100
    } else {
        s
    }
}

/// Vertical displacement from the ground `n` ticks after a jump press
/// (positive = below ground clamp; the player is airborne while `< 0`).
///
/// `d(n) = Σ_{j=1..n} vy_j` with `vy_j = min(v0 + G·j, cap)`; without cape
/// the cap is +∞. Parabola: `P(m) = m·v0 + (G/2)·m·(m+1)`.
pub fn jump_disp(v0: i32, cape: bool, n: u32) -> i64 {
    let n = i64::from(n);
    let v0 = i64::from(v0);
    let g = i64::from(GRAVITY);
    let parab = |m: i64| m * v0 + (g / 2) * m * (m + 1);
    if !cape {
        return parab(n);
    }
    // First tick where v0 + G·n >= GLIDE_CAP.
    let cap = i64::from(GLIDE_CAP);
    let nc = (cap - v0 + g - 1).div_euclid(g); // ceil((cap - v0)/g)
    if n < nc {
        parab(n)
    } else {
        parab(nc - 1) + (n - nc + 1) * cap
    }
}

/// Number of ticks until a jump lands: smallest `n ≥ 1` with `d(n) ≥ 0`.
pub fn jump_landing(v0: i32, cape: bool) -> u32 {
    let mut n = 1u32;
    while jump_disp(v0, cape, n) < 0 {
        n += 1;
    }
    n
}

/// Bat hover offset (fp) at `p = phase0 + (t - spawn_tick)`.
pub fn bat_offset(p: u32) -> i32 {
    let p = (p % BAT_PERIOD) as i32;
    let half = (BAT_PERIOD / 2) as i32;
    let q = if p < half { p } else { BAT_PERIOD as i32 - p };
    (2 * q - half) * BAT_STEP
}

/// xorshift64* PRNG (identical to `dash_core`).
#[derive(Clone, Copy)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: u32) -> u32 {
        (self.next() % u64::from(n)) as u32
    }
}

// --- Seed-derived schedule (computed natively, bound as public inputs) ---

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct GroundObs {
    pub spawn_tick: u32,
    /// 0 = barrel, 1 = pipe.
    pub kind: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct BatObs {
    pub spawn_tick: u32,
    /// Hover-center top edge, px.
    pub base_y_px: i32,
    pub phase0: u32,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct SchedItem {
    pub spawn_tick: u32,
    pub kind: i32,
    /// Top edge, px.
    pub y_px: i32,
}

/// The full obstacle/item timeline for one seed, split by kind so the
/// circuit can use cheaper templates for ground obstacles than for bats.
#[derive(Clone)]
pub struct Schedule {
    pub ground: [GroundObs; MAX_GROUND],
    pub ground_count: usize,
    pub bats: [BatObs; MAX_BATS],
    pub bat_count: usize,
    pub items: [SchedItem; MAX_SCHED_ITEMS],
    pub item_count: usize,
}

impl Schedule {
    /// Derives the schedule for `seed`. Mirrors `dash_core` spawn logic at
    /// 30 Hz. RNG call order is fixed; capacity overflow skips the spawn
    /// (never reached in a 2-minute run in practice).
    pub fn generate(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut sched = Schedule {
            ground: [GroundObs::default(); MAX_GROUND],
            ground_count: 0,
            bats: [BatObs::default(); MAX_BATS],
            bat_count: 0,
            items: [SchedItem::default(); MAX_SCHED_ITEMS],
            item_count: 0,
        };
        let mut spawn_in: u32 = 42; // 1.4 s
        let mut item_in: u32 = 48; // 1.6 s
        for t in 1..=MAX_TICKS {
            if spawn_in > 0 {
                spawn_in -= 1;
            }
            if spawn_in == 0 {
                let roll = rng.below(100);
                if roll < 60 {
                    let w = 34 + rng.below(19) as i32;
                    let h = 42 + rng.below(35) as i32;
                    if sched.ground_count < MAX_GROUND {
                        sched.ground[sched.ground_count] = GroundObs {
                            spawn_tick: t,
                            kind: KIND_BARREL,
                            w,
                            h,
                        };
                        sched.ground_count += 1;
                    }
                } else if roll < 85 {
                    let h = 84 + rng.below(31) as i32;
                    if sched.ground_count < MAX_GROUND {
                        sched.ground[sched.ground_count] = GroundObs {
                            spawn_tick: t,
                            kind: KIND_PIPE,
                            w: 46,
                            h,
                        };
                        sched.ground_count += 1;
                    }
                } else {
                    let base_y_px = GROUND_Y - 90 - rng.below(71) as i32;
                    let phase0 = rng.below(BAT_PERIOD);
                    if sched.bat_count < MAX_BATS {
                        sched.bats[sched.bat_count] = BatObs {
                            spawn_tick: t,
                            base_y_px,
                            phase0,
                        };
                        sched.bat_count += 1;
                    }
                }
                let gap = 22 + rng.below(28);
                spawn_in = ((i64::from(gap) * SPEED_BASE100) / s100(t)) as u32 + 8;
            }
            if item_in > 0 {
                item_in -= 1;
            }
            if item_in == 0 {
                let kind = rng.below(3) as i32;
                let high = rng.below(2) == 1;
                let y_px = if high {
                    GROUND_Y - 150 - rng.below(61) as i32
                } else {
                    GROUND_Y - 46
                };
                if sched.item_count < MAX_SCHED_ITEMS {
                    sched.items[sched.item_count] = SchedItem {
                        spawn_tick: t,
                        kind,
                        y_px,
                    };
                    sched.item_count += 1;
                }
                item_in = 66 + rng.below(97);
            }
        }
        sched
    }
}

/// Left edge (fp100) at tick `t` of a world entity spawned at `spawn_tick`
/// from `x0_100` (entities are moved on their spawn tick).
pub fn world_x100(x0_100: i64, spawn_tick: u32, t: u32) -> i64 {
    x0_100 - (d100(t) - d100(spawn_tick - 1))
}

/// Fireball left edge (fp100) at tick `t` for a fireball fired at `tf`
/// (moved on its fire tick).
pub fn fireball_x100(tf: u32, t: u32) -> i64 {
    FIREBALL_X0_100 + i64::from(t + 1 - tf) * FIREBALL_SPEED100
}

/// Bat top edge (fp) at tick `t`.
pub fn bat_y_fp(b: &BatObs, t: u32) -> i32 {
    b.base_y_px * FP + bat_offset(b.phase0 + (t - b.spawn_tick))
}

// --- Recorded events (the ZK witness skeleton) ---

#[derive(Clone, Copy, Default, Debug)]
pub struct JumpEv {
    pub tick: u32,
    pub v0: i32,
    pub cape: bool,
    /// Ticks until landing (redundant; witness convenience).
    pub n_land: u32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct PickupEv {
    pub tick: u32,
    pub item_idx: u32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct KillEv {
    pub fire_tick: u32,
    pub hit_tick: u32,
    /// 0 = ground obstacle, 1 = bat.
    pub target_class: u32,
    pub target_idx: u32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct DamageEv {
    pub tick: u32,
    /// 0 = ground obstacle, 1 = bat, 2 = invulnerable touch (no damage).
    pub class: u32,
    pub idx: u32,
    pub invuln_touch: bool,
}

/// Per-obstacle terminal status after a run (for witness building).
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum ObsStatus {
    /// Never touched: clearance must be proven over its overlap window.
    #[default]
    Cleared,
    /// Destroyed by fireball at tick.
    Killed(u32),
    /// Collided at tick (damage taken).
    Damaged(u32),
    /// Touched while invulnerable at tick (destroyed, no damage).
    InvulnTouch(u32),
}

const MAX_FIRES_TRACKED: usize = 4;

/// The playable simulation. JavaScript renders it; its recorded events plus
/// the schedule are exactly the circuit witness.
#[derive(Clone)]
pub struct ZkSim {
    pub seed: u64,
    sched: Schedule,
    ticks: u32,
    over: bool,
    form: DarioState,
    score: u64,
    pickups_n: u32,
    kills_n: u32,
    prev_input: u8,
    // Player: current jump (index into jumps) or grounded.
    cur_jump: Option<usize>,
    last_fire: Option<u32>,
    last_damage: Option<u32>,
    // Entity liveness.
    ground_status: [ObsStatus; MAX_GROUND],
    bat_status: [ObsStatus; MAX_BATS],
    item_taken: [bool; MAX_SCHED_ITEMS],
    // In-flight fireballs: fire tick + top edge (fp), plus kill flag.
    fires: [(u32, i32, bool); MAX_FIRES_TRACKED],
    fire_count: usize,
    // Event log.
    pub jumps: [JumpEv; MAX_JUMPS],
    pub jump_count: usize,
    pub pickups: [PickupEv; MAX_PICKUPS],
    pub pickup_count: usize,
    pub kills: [KillEv; MAX_KILLS],
    pub kill_count: usize,
    pub damages: [DamageEv; MAX_DAMAGES + MAX_GROUND],
    pub damage_count: usize,
}

impl ZkSim {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            sched: Schedule::generate(seed),
            ticks: 0,
            over: false,
            form: DarioState::Regular,
            score: 0,
            pickups_n: 0,
            kills_n: 0,
            prev_input: 0,
            cur_jump: None,
            last_fire: None,
            last_damage: None,
            ground_status: [ObsStatus::Cleared; MAX_GROUND],
            bat_status: [ObsStatus::Cleared; MAX_BATS],
            item_taken: [false; MAX_SCHED_ITEMS],
            fires: [(0, 0, false); MAX_FIRES_TRACKED],
            fire_count: 0,
            jumps: [JumpEv::default(); MAX_JUMPS],
            jump_count: 0,
            pickups: [PickupEv::default(); MAX_PICKUPS],
            pickup_count: 0,
            kills: [KillEv::default(); MAX_KILLS],
            kill_count: 0,
            damages: [DamageEv::default(); MAX_DAMAGES + MAX_GROUND],
            damage_count: 0,
        }
    }

    pub fn schedule(&self) -> &Schedule {
        &self.sched
    }

    fn grounded_at(&self, t: u32) -> bool {
        match self.cur_jump {
            None => true,
            Some(j) => {
                let jump = &self.jumps[j];
                t >= jump.tick + jump.n_land
            }
        }
    }

    /// Player feet y (fp) at tick `t` (uses the current-jump closed form).
    fn player_y_fp(&self, t: u32) -> i32 {
        match self.cur_jump {
            None => GROUND_Y * FP,
            Some(j) => {
                let jump = &self.jumps[j];
                if t >= jump.tick + jump.n_land || t < jump.tick {
                    GROUND_Y * FP
                } else {
                    let n = (t - jump.tick).max(1);
                    GROUND_Y * FP + jump_disp(jump.v0, jump.cape, n) as i32
                }
            }
        }
    }

    fn protected(&self, t: u32) -> bool {
        match self.last_damage {
            Some(td) => t >= td + 1 && t <= td + INVULN_TICKS,
            None => false,
        }
    }

    /// Advance one tick. Input bits: 1 = jump, 2 = fire.
    pub fn tick(&mut self, input: u8) {
        if self.over || self.ticks >= MAX_TICKS {
            self.over = true;
            return;
        }
        let input = input & INPUT_MASK;
        let pressed = input & !self.prev_input;
        self.prev_input = input;

        self.ticks += 1;
        let t = self.ticks;

        // Jump (edge-triggered, only when grounded).
        if pressed & INPUT_JUMP != 0 && self.grounded_at(t) && self.jump_count < MAX_JUMPS {
            let v0 = if self.form == DarioState::Super {
                SUPER_JUMP_V
            } else {
                JUMP_V
            };
            let cape = self.form == DarioState::Cape;
            let n_land = jump_landing(v0, cape);
            self.jumps[self.jump_count] = JumpEv {
                tick: t,
                v0,
                cape,
                n_land,
            };
            self.cur_jump = Some(self.jump_count);
            self.jump_count += 1;
        }
        // On the press tick (n = 0) dash_core ordering means the player has
        // already moved by vy_1, so evaluate the closed form with n >= 1.
        let py = if let Some(j) = self.cur_jump {
            let jump = self.jumps[j];
            if t >= jump.tick && t < jump.tick + jump.n_land {
                let n = (t - jump.tick).max(1);
                GROUND_Y * FP + jump_disp(jump.v0, jump.cape, n) as i32
            } else {
                if t >= jump.tick + jump.n_land {
                    self.cur_jump = None;
                }
                GROUND_Y * FP
            }
        } else {
            GROUND_Y * FP
        };

        // Fire (edge-triggered, Fire form, cooldown).
        if pressed & INPUT_FIRE != 0
            && self.form == DarioState::Fire
            && self
                .last_fire
                .map_or(true, |tf| t >= tf + FIREBALL_COOLDOWN)
        {
            // Drop expired fireballs to free slots (capacity never binds:
            // lifetime 39 < 3 * cooldown 14 is false, but 4 slots suffice).
            self.compact_fires(t);
            if self.fire_count < MAX_FIRES_TRACKED {
                let fb_y = py - (PLAYER_H * 55 / 100) * FP;
                self.fires[self.fire_count] = (t, fb_y, false);
                self.fire_count += 1;
                self.last_fire = Some(t);
            }
        }

        // Item pickups (auto-collect on overlap).
        let (ptop, pbot) = (py - PLAYER_BOX_TOP_FP, py - PLAYER_BOX_BOT_FP);
        for i in 0..self.sched.item_count {
            if self.item_taken[i] {
                continue;
            }
            let it = self.sched.items[i];
            if t < it.spawn_tick {
                break;
            }
            let x = world_x100(ITEM_X0_100, it.spawn_tick, t);
            if x + (ITEM_SIZE as i64) * FP100 < -40 * FP100 {
                continue;
            }
            let x_overlap = x < PLAYER_RIGHT100 && x + (ITEM_SIZE as i64) * FP100 > PLAYER_LEFT100;
            let iy = it.y_px * FP;
            let y_overlap = ptop < iy + ITEM_SIZE * FP && pbot > iy;
            if x_overlap && y_overlap && self.pickup_count < MAX_PICKUPS {
                self.item_taken[i] = true;
                self.pickups[self.pickup_count] = PickupEv {
                    tick: t,
                    item_idx: i as u32,
                };
                self.pickup_count += 1;
                self.pickups_n += 1;
                let ev = match it.kind {
                    k if k == ITEM_CHILI => Event::ChiliPepper,
                    k if k == ITEM_CAPE => Event::TableClothCape,
                    _ => Event::Espresso,
                };
                self.form = transition(self.form, ev);
            }
        }

        // Fireball vs obstacles.
        for fi in 0..self.fire_count {
            let (tf, fy, dead) = self.fires[fi];
            if dead || t + 1 - tf > FIREBALL_LIFE + 1 {
                continue;
            }
            let fx = fireball_x100(tf, t);
            if fx > OBS_X0_100 {
                continue;
            }
            let fright = fx + (FIREBALL_SIZE as i64) * FP100;
            let fbot = fy + FIREBALL_SIZE * FP;
            let mut hit: Option<(u32, u32)> = None;
            for gi in 0..self.sched.ground_count {
                if self.ground_status[gi] != ObsStatus::Cleared {
                    continue;
                }
                let g = self.sched.ground[gi];
                if t < g.spawn_tick {
                    break;
                }
                let (l, r, top, bot) = ground_hitbox(&g, t);
                if fx < r && fright > l && fy < bot && fbot > top {
                    hit = Some((0, gi as u32));
                    break;
                }
            }
            if hit.is_none() {
                for bi in 0..self.sched.bat_count {
                    if self.bat_status[bi] != ObsStatus::Cleared {
                        continue;
                    }
                    let b = self.sched.bats[bi];
                    if t < b.spawn_tick {
                        break;
                    }
                    let (l, r, top, bot) = bat_hitbox(&b, t);
                    if fx < r && fright > l && fy < bot && fbot > top {
                        hit = Some((1, bi as u32));
                        break;
                    }
                }
            }
            if let Some((class, idx)) = hit {
                // Past the kill-event capacity the circuit cannot express a
                // killed obstacle, so the fireball passes through harmlessly
                // (mirrors the jump-cap handling: suppress, don't desync).
                if self.kill_count < MAX_KILLS {
                    self.fires[fi].2 = true;
                    if class == 0 {
                        self.ground_status[idx as usize] = ObsStatus::Killed(t);
                    } else {
                        self.bat_status[idx as usize] = ObsStatus::Killed(t);
                    }
                    self.kills[self.kill_count] = KillEv {
                        fire_tick: tf,
                        hit_tick: t,
                        target_class: class,
                        target_idx: idx,
                    };
                    self.kill_count += 1;
                    self.kills_n += 1;
                }
            }
        }

        // Player vs obstacles.
        let mut collide: Option<(u32, u32)> = None;
        for gi in 0..self.sched.ground_count {
            if self.ground_status[gi] != ObsStatus::Cleared {
                continue;
            }
            let g = self.sched.ground[gi];
            if t < g.spawn_tick {
                break;
            }
            let (l, r, top, bot) = ground_hitbox(&g, t);
            if PLAYER_LEFT100 < r && PLAYER_RIGHT100 > l && ptop < bot && pbot > top {
                collide = Some((0, gi as u32));
                break;
            }
        }
        if collide.is_none() {
            for bi in 0..self.sched.bat_count {
                if self.bat_status[bi] != ObsStatus::Cleared {
                    continue;
                }
                let b = self.sched.bats[bi];
                if t < b.spawn_tick {
                    break;
                }
                let (l, r, top, bot) = bat_hitbox(&b, t);
                if PLAYER_LEFT100 < r && PLAYER_RIGHT100 > l && ptop < bot && pbot > top {
                    collide = Some((1, bi as u32));
                    break;
                }
            }
        }
        if let Some((class, idx)) = collide {
            // Past the damage-event capacity the circuit cannot express a
            // touched obstacle, so the collision is ignored entirely to keep
            // the sim and witness consistent (same rationale as the kill cap).
            if self.damage_count < self.damages.len() {
                if self.protected(t) {
                    let status = ObsStatus::InvulnTouch(t);
                    if class == 0 {
                        self.ground_status[idx as usize] = status;
                    } else {
                        self.bat_status[idx as usize] = status;
                    }
                    self.push_damage(t, class, idx, true);
                } else {
                    let status = ObsStatus::Damaged(t);
                    if class == 0 {
                        self.ground_status[idx as usize] = status;
                    } else {
                        self.bat_status[idx as usize] = status;
                    }
                    self.push_damage(t, class, idx, false);
                    let was = self.form;
                    self.form = transition(was, Event::TakeDamage);
                    if self.form == DarioState::GameOver {
                        self.over = true;
                    } else {
                        self.last_damage = Some(t);
                    }
                }
            }
        }

        // Score.
        self.score = (d100(t) / (i64::from(FP) * 100 * 50)) as u64
            + u64::from(self.pickups_n) * 50
            + u64::from(self.kills_n) * 25;

        if self.ticks >= MAX_TICKS {
            self.over = true;
        }
    }

    fn push_damage(&mut self, t: u32, class: u32, idx: u32, invuln: bool) {
        if self.damage_count < self.damages.len() {
            self.damages[self.damage_count] = DamageEv {
                tick: t,
                class,
                idx,
                invuln_touch: invuln,
            };
            self.damage_count += 1;
        }
    }

    fn compact_fires(&mut self, t: u32) {
        let mut w = 0;
        for r in 0..self.fire_count {
            let (tf, y, dead) = self.fires[r];
            if !dead && t + 1 - tf <= FIREBALL_LIFE {
                self.fires[w] = (tf, y, dead);
                w += 1;
            }
        }
        self.fire_count = w;
    }

    // --- Accessors ---

    pub fn over(&self) -> bool {
        self.over
    }

    pub fn score(&self) -> u64 {
        self.score
    }

    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    pub fn form(&self) -> u32 {
        self.form as u32
    }

    pub fn pickups_total(&self) -> u32 {
        self.pickups_n
    }

    pub fn kills_total(&self) -> u32 {
        self.kills_n
    }

    pub fn distance_px(&self) -> u64 {
        (d100(self.ticks) / (i64::from(FP) * 100)) as u64
    }

    pub fn invulnerable(&self) -> bool {
        self.protected(self.ticks + 1)
    }

    pub fn grounded(&self) -> bool {
        self.grounded_at(self.ticks)
    }

    pub fn player_px(&self) -> (i32, i32, i32, i32) {
        (
            PLAYER_X,
            self.player_y_fp(self.ticks) / FP,
            PLAYER_W,
            PLAYER_H,
        )
    }

    pub fn ground_status(&self) -> &[ObsStatus; MAX_GROUND] {
        &self.ground_status
    }

    pub fn bat_status(&self) -> &[ObsStatus; MAX_BATS] {
        &self.bat_status
    }

    pub fn item_taken(&self) -> &[bool; MAX_SCHED_ITEMS] {
        &self.item_taken
    }

    /// Render snapshot, same layout as `dash_core`:
    /// `[entity_type, kind, x_px, y_px, w_px, h_px]` records; entity_type
    /// 0 = obstacle (y = ground anchor for barrel/pipe, top for bat),
    /// 1 = item, 2 = fireball.
    pub fn snapshot(&self, out: &mut [i32]) -> usize {
        let t = self.ticks;
        let mut n = 0;
        let mut push = |out: &mut [i32], rec: [i32; 6]| {
            if n + 6 <= out.len() {
                out[n..n + 6].copy_from_slice(&rec);
                n += 6;
            }
        };
        for gi in 0..self.sched.ground_count {
            if self.ground_status[gi] != ObsStatus::Cleared {
                continue;
            }
            let g = self.sched.ground[gi];
            if t < g.spawn_tick {
                break;
            }
            let x = world_x100(OBS_X0_100, g.spawn_tick, t);
            if x + (g.w as i64) * FP100 < -60 * FP100 {
                continue;
            }
            push(out, [0, g.kind, (x / FP100) as i32, GROUND_Y, g.w, g.h]);
        }
        for bi in 0..self.sched.bat_count {
            if self.bat_status[bi] != ObsStatus::Cleared {
                continue;
            }
            let b = self.sched.bats[bi];
            if t < b.spawn_tick {
                break;
            }
            let x = world_x100(OBS_X0_100, b.spawn_tick, t);
            if x + (BAT_W as i64) * FP100 < -60 * FP100 {
                continue;
            }
            push(
                out,
                [
                    0,
                    KIND_BAT,
                    (x / FP100) as i32,
                    bat_y_fp(&b, t) / FP,
                    BAT_W,
                    BAT_H,
                ],
            );
        }
        for i in 0..self.sched.item_count {
            if self.item_taken[i] {
                continue;
            }
            let it = self.sched.items[i];
            if t < it.spawn_tick {
                break;
            }
            let x = world_x100(ITEM_X0_100, it.spawn_tick, t);
            if x + (ITEM_SIZE as i64) * FP100 < -40 * FP100 {
                continue;
            }
            push(
                out,
                [
                    1,
                    it.kind,
                    (x / FP100) as i32,
                    it.y_px,
                    ITEM_SIZE,
                    ITEM_SIZE,
                ],
            );
        }
        for fi in 0..self.fire_count {
            let (tf, fy, dead) = self.fires[fi];
            if dead || t + 1 - tf > FIREBALL_LIFE {
                continue;
            }
            let x = fireball_x100(tf, t);
            push(
                out,
                [
                    2,
                    0,
                    (x / FP100) as i32,
                    fy / FP,
                    FIREBALL_SIZE,
                    FIREBALL_SIZE,
                ],
            );
        }
        n
    }
}

/// Ground obstacle hitbox at tick `t`: (left100, right100, top_fp, bot_fp).
pub fn ground_hitbox(g: &GroundObs, t: u32) -> (i64, i64, i32, i32) {
    let x = world_x100(OBS_X0_100, g.spawn_tick, t);
    (
        x + 4 * FP100,
        x + (g.w as i64 - 4) * FP100,
        (GROUND_Y - g.h + 4) * FP,
        GROUND_Y * FP,
    )
}

/// Bat hitbox at tick `t`: (left100, right100, top_fp, bot_fp).
pub fn bat_hitbox(b: &BatObs, t: u32) -> (i64, i64, i32, i32) {
    let x = world_x100(OBS_X0_100, b.spawn_tick, t);
    let y = bat_y_fp(b, t);
    (x, x + (BAT_W as i64) * FP100, y, y + BAT_H * FP)
}

/// Replay a per-tick input trace; returns (score, ticks, over).
pub fn replay(seed: u64, trace: &[u8]) -> (u64, u32, bool) {
    let mut sim = ZkSim::new(seed);
    for &b in trace {
        if sim.over() {
            break;
        }
        sim.tick(b);
    }
    (sim.score(), sim.ticks(), sim.over())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d100_matches_iterative_sum() {
        let mut acc = 0i64;
        for t in 1..=MAX_TICKS {
            acc += s100(t);
            assert_eq!(d100(t), acc, "tick {t}");
        }
    }

    #[test]
    fn jump_disp_matches_per_tick_integration() {
        for &(v0, cape) in &[
            (JUMP_V, false),
            (SUPER_JUMP_V, false),
            (JUMP_V, true),
            (SUPER_JUMP_V, true),
        ] {
            let mut d = 0i64;
            for n in 1..200u32 {
                let mut vy = i64::from(v0) + i64::from(GRAVITY) * i64::from(n);
                if cape && vy > i64::from(GLIDE_CAP) {
                    vy = i64::from(GLIDE_CAP);
                }
                d += vy;
                assert_eq!(jump_disp(v0, cape, n), d, "v0={v0} cape={cape} n={n}");
            }
        }
    }

    #[test]
    fn landing_is_first_nonnegative() {
        for &(v0, cape) in &[
            (JUMP_V, false),
            (SUPER_JUMP_V, false),
            (JUMP_V, true),
            (SUPER_JUMP_V, true),
        ] {
            let n = jump_landing(v0, cape);
            assert!(jump_disp(v0, cape, n) >= 0);
            assert!(jump_disp(v0, cape, n - 1) < 0);
        }
    }

    /// The circuit hard-codes these landing times per form combo:
    /// n_land = 21 + 3*is_super + 20*is_cape + 11*is_super*is_cape,
    /// and glide crossover ticks nc = 13 + 2*is_super with linear-tail
    /// offsets parab(nc-1) = -36480 / -51324.
    #[test]
    fn landing_constants_match_circuit() {
        assert_eq!(jump_landing(JUMP_V, false), 21);
        assert_eq!(jump_landing(SUPER_JUMP_V, false), 24);
        assert_eq!(jump_landing(JUMP_V, true), 41);
        assert_eq!(jump_landing(SUPER_JUMP_V, true), 55);
        assert_eq!(jump_disp(JUMP_V, true, 12), -36480);
        assert_eq!(jump_disp(SUPER_JUMP_V, true, 14), -51324);
    }

    #[test]
    fn schedule_is_deterministic_and_ordered() {
        let a = Schedule::generate(42);
        let b = Schedule::generate(42);
        assert_eq!(a.ground_count, b.ground_count);
        assert_eq!(a.bat_count, b.bat_count);
        assert_eq!(a.item_count, b.item_count);
        assert!(a.ground_count > 0 && a.item_count > 0);
        for i in 1..a.ground_count {
            assert!(a.ground[i].spawn_tick > a.ground[i - 1].spawn_tick);
        }
        for i in 1..a.item_count {
            assert!(a.items[i].spawn_tick > a.items[i - 1].spawn_tick);
        }
    }

    #[test]
    fn idle_run_dies_or_caps() {
        let (score, ticks, over) = replay(42, &[0u8; MAX_TICKS as usize]);
        assert!(over);
        assert!(ticks > 0 && ticks <= MAX_TICKS);
        let _ = score;
    }

    #[test]
    fn autopilot_survives_longer_than_idle() {
        // Jump whenever a ground obstacle approaches; crude but effective.
        let mut sim = ZkSim::new(7);
        let mut idle = ZkSim::new(7);
        let mut snap = [0i32; 36 * 6];
        let mut prev = 0u8;
        while !sim.over() && sim.ticks() < MAX_TICKS {
            let n = sim.snapshot(&mut snap);
            let mut want_jump = false;
            for rec in snap[..n].chunks(6) {
                if rec[0] == 0 && rec[1] != KIND_BAT {
                    let dist = rec[2] - (PLAYER_X + PLAYER_W);
                    if (0..=60).contains(&dist) {
                        want_jump = true;
                    }
                }
            }
            let input = if want_jump && prev & INPUT_JUMP == 0 {
                INPUT_JUMP
            } else {
                0
            };
            prev = input;
            sim.tick(input);
        }
        while !idle.over() {
            idle.tick(0);
        }
        assert!(
            sim.ticks() > idle.ticks(),
            "{} vs {}",
            sim.ticks(),
            idle.ticks()
        );
        assert!(sim.jump_count > 0);
    }

    #[test]
    fn max_ticks_run_is_capped() {
        // Autopilot from previous test may die; just check tick cap logic.
        let mut sim = ZkSim::new(3);
        for _ in 0..(MAX_TICKS + 10) {
            sim.tick(0);
            if sim.over() {
                break;
            }
        }
        assert!(sim.ticks() <= MAX_TICKS);
    }
}
