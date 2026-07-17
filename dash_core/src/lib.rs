//! # Dario Dash — deterministic simulation core
//!
//! Pure-integer, fixed-point (`1 px == 256 fp`), fixed-timestep (60 Hz)
//! simulation of the Dario Dash endless runner. The exact same code runs:
//!
//! - in the browser (compiled to WASM via `dash_web`), where JavaScript is
//!   only a renderer and input recorder, and
//! - inside the RISC Zero zkVM guest, which replays a recorded input trace
//!   to prove the achieved score.
//!
//! Determinism rules: no floats, no ambient randomness, no time sources.
//! All randomness comes from a seeded xorshift64* PRNG.
#![no_std]

use dario_fsm::{transition, DarioState, Event};

/// Simulation tick rate in Hz.
pub const TICK_HZ: u32 = 60;
/// Fixed-point scale: 1 pixel = 256 fp units.
pub const FP: i32 = 256;

/// World dimensions in pixels.
pub const WORLD_W: i32 = 960;
pub const WORLD_H: i32 = 540;
/// Ground line (player feet) in pixels.
pub const GROUND_Y: i32 = 464;

/// Input bitflags, one byte per tick.
pub const INPUT_JUMP: u8 = 1;
pub const INPUT_FIRE: u8 = 2;
/// Only these bits are meaningful; traces must not set others.
pub const INPUT_MASK: u8 = INPUT_JUMP | INPUT_FIRE;

/// Hard cap on run length (10 minutes) so proofs stay bounded.
pub const MAX_TICKS: u32 = TICK_HZ * 600;

// Physics constants, converted from px/s (px/s²) to fp/tick (fp/tick²).
const GRAVITY: i32 = 185; // 2600 px/s²
const JUMP_V: i32 = -3925; // -920 px/s
const SUPER_JUMP_V: i32 = -4608; // -1080 px/s
const GLIDE_FALL_CAP: i32 = 640; // 150 px/s
const BASE_SPEED: i32 = 1408; // 330 px/s
const MAX_SPEED: i32 = 3243; // 760 px/s
const FIREBALL_SPEED: i32 = 2645; // 620 px/s
const FIREBALL_COOLDOWN: u32 = 27; // 0.45 s
const INVULN_TICKS: u32 = 78; // 1.3 s

const PLAYER_X: i32 = 130;
const PLAYER_W: i32 = 46;
const PLAYER_H: i32 = 74;

pub const MAX_OBSTACLES: usize = 16;
pub const MAX_ITEMS: usize = 8;
pub const MAX_FIREBALLS: usize = 4;

/// Entity kinds exposed in snapshots.
pub const KIND_BARREL: i32 = 0;
pub const KIND_PIPE: i32 = 1;
pub const KIND_BAT: i32 = 2;

/// Item kinds (map 1:1 to FSM events).
pub const ITEM_ESPRESSO: i32 = 0;
pub const ITEM_CHILI: i32 = 1;
pub const ITEM_CAPE: i32 = 2;

#[derive(Clone, Copy, Default)]
struct Obstacle {
    active: bool,
    kind: i32,
    /// Left edge, fp.
    x: i32,
    /// Barrels/pipes: ground anchor (bottom). Bats: top edge, fp.
    y: i32,
    /// Pixels.
    w: i32,
    h: i32,
    /// Bats: base y (fp) for the triangle-wave hover.
    base_y: i32,
    /// Bats: hover phase in ticks.
    phase: u32,
}

#[derive(Clone, Copy, Default)]
struct Item {
    active: bool,
    kind: i32,
    x: i32,
    y: i32,
}

const ITEM_SIZE: i32 = 34;

#[derive(Clone, Copy, Default)]
struct Fireball {
    active: bool,
    x: i32,
    y: i32,
}

const FIREBALL_SIZE: i32 = 18;

/// xorshift64* PRNG.
#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid the all-zero fixed point.
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Uniform-ish value in `0..n` (n > 0).
    fn below(&mut self, n: u32) -> u32 {
        (self.next() % u64::from(n)) as u32
    }
}

/// Triangle wave in `-amp..=amp` with the given period, for bat hovering.
fn triangle(phase: u32, period: u32, amp: i32) -> i32 {
    let p = (phase % period) as i32;
    let half = (period / 2) as i32;
    let q = if p < half { p } else { (period as i32) - p };
    // q in 0..=half
    (2 * q - half) * amp / half
}

/// The full deterministic game state.
#[derive(Clone)]
pub struct Sim {
    rng: Rng,
    ticks: u32,
    over: bool,
    form: DarioState,
    score: u64,
    pickups: u32,
    kills: u32,
    /// Scrolled distance, fp.
    distance: i64,
    /// Player feet position (fp) and vertical velocity (fp/tick).
    player_y: i32,
    player_vy: i32,
    grounded: bool,
    invuln: u32,
    fire_cd: u32,
    prev_input: u8,
    spawn_in: u32,
    item_in: u32,
    obstacles: [Obstacle; MAX_OBSTACLES],
    items: [Item; MAX_ITEMS],
    fireballs: [Fireball; MAX_FIREBALLS],
}

/// Result of replaying a full input trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunResult {
    pub score: u64,
    pub ticks: u32,
    pub over: bool,
}

struct Aabb {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn overlap(a: &Aabb, b: &Aabb) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

impl Sim {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
            ticks: 0,
            over: false,
            form: DarioState::Regular,
            score: 0,
            pickups: 0,
            kills: 0,
            distance: 0,
            player_y: GROUND_Y * FP,
            player_vy: 0,
            grounded: true,
            invuln: 0,
            fire_cd: 0,
            prev_input: 0,
            spawn_in: 84, // 1.4 s
            item_in: 96,  // 1.6 s
            obstacles: [Obstacle::default(); MAX_OBSTACLES],
            items: [Item::default(); MAX_ITEMS],
            fireballs: [Fireball::default(); MAX_FIREBALLS],
        }
    }

    /// Current horizontal scroll speed, fp/tick.
    fn speed(&self) -> i32 {
        let s = BASE_SPEED + ((self.ticks as i64 * 64) / 100) as i32;
        if s > MAX_SPEED {
            MAX_SPEED
        } else {
            s
        }
    }

    fn apply_event(&mut self, event: Event) {
        self.form = transition(self.form, event);
    }

    fn spawn_obstacle(&mut self) {
        let slot = match self.obstacles.iter().position(|o| !o.active) {
            Some(i) => i,
            None => return,
        };
        let roll = self.rng.below(100);
        let o = &mut self.obstacles[slot];
        o.active = true;
        o.x = (WORLD_W + 40) * FP;
        o.phase = 0;
        if roll < 60 {
            o.kind = KIND_BARREL;
            o.w = 34 + self.rng.below(19) as i32;
            o.h = 42 + self.rng.below(35) as i32;
            o.y = GROUND_Y * FP;
        } else if roll < 85 {
            o.kind = KIND_PIPE;
            o.w = 46;
            o.h = 84 + self.rng.below(31) as i32;
            o.y = GROUND_Y * FP;
        } else {
            o.kind = KIND_BAT;
            o.w = 40;
            o.h = 32;
            o.base_y = (GROUND_Y - 90 - self.rng.below(71) as i32) * FP;
            o.y = o.base_y;
            o.phase = self.rng.below(72);
        }
    }

    fn spawn_item(&mut self) {
        let slot = match self.items.iter().position(|i| !i.active) {
            Some(i) => i,
            None => return,
        };
        let kind = self.rng.below(3) as i32;
        let high = self.rng.below(2) == 1;
        let y = if high {
            GROUND_Y - 150 - self.rng.below(61) as i32
        } else {
            GROUND_Y - 46
        };
        let it = &mut self.items[slot];
        it.active = true;
        it.kind = kind;
        it.x = (WORLD_W + 30) * FP;
        it.y = y * FP;
    }

    fn player_hitbox(&self) -> Aabb {
        Aabb {
            x: (PLAYER_X + 8) * FP,
            y: self.player_y - (PLAYER_H - 6) * FP,
            w: (PLAYER_W - 16) * FP,
            h: (PLAYER_H - 10) * FP,
        }
    }

    fn obstacle_hitbox(o: &Obstacle) -> Aabb {
        if o.kind == KIND_BAT {
            Aabb {
                x: o.x,
                y: o.y,
                w: o.w * FP,
                h: o.h * FP,
            }
        } else {
            Aabb {
                x: o.x + 4 * FP,
                y: o.y - (o.h - 4) * FP,
                w: (o.w - 8) * FP,
                h: (o.h - 4) * FP,
            }
        }
    }

    fn hit_player(&mut self) {
        if self.invuln > 0 {
            return;
        }
        self.apply_event(Event::TakeDamage);
        if self.form == DarioState::GameOver {
            self.over = true;
        } else {
            self.invuln = INVULN_TICKS;
        }
    }

    /// Advance the simulation one tick with the given input byte.
    pub fn tick(&mut self, input: u8) {
        if self.over || self.ticks >= MAX_TICKS {
            self.over = true;
            return;
        }
        let input = input & INPUT_MASK;
        let pressed = input & !self.prev_input;
        self.prev_input = input;

        self.ticks += 1;
        let speed = self.speed();
        self.distance += i64::from(speed);
        // score: 0.02 points per scrolled pixel -> distance_fp / (256 * 50)
        self.score = (self.distance / (i64::from(FP) * 50)) as u64
            + u64::from(self.pickups) * 50
            + u64::from(self.kills) * 25;

        if self.invuln > 0 {
            self.invuln -= 1;
        }
        if self.fire_cd > 0 {
            self.fire_cd -= 1;
        }

        // Player physics
        if pressed & INPUT_JUMP != 0 && self.grounded {
            self.player_vy = if self.form == DarioState::Super {
                SUPER_JUMP_V
            } else {
                JUMP_V
            };
            self.grounded = false;
        }
        self.player_vy += GRAVITY;
        if self.form == DarioState::Cape
            && input & INPUT_JUMP != 0
            && self.player_vy > GLIDE_FALL_CAP
        {
            self.player_vy = GLIDE_FALL_CAP;
        }
        self.player_y += self.player_vy;
        if self.player_y >= GROUND_Y * FP {
            self.player_y = GROUND_Y * FP;
            self.player_vy = 0;
            self.grounded = true;
        }

        // Fireballs
        if pressed & INPUT_FIRE != 0 && self.form == DarioState::Fire && self.fire_cd == 0 {
            if let Some(slot) = self.fireballs.iter().position(|f| !f.active) {
                self.fire_cd = FIREBALL_COOLDOWN;
                self.fireballs[slot] = Fireball {
                    active: true,
                    x: (PLAYER_X + PLAYER_W) * FP,
                    y: self.player_y - (PLAYER_H * 55 / 100) * FP,
                };
            }
        }

        // Spawns
        if self.spawn_in > 0 {
            self.spawn_in -= 1;
        }
        if self.spawn_in == 0 {
            self.spawn_obstacle();
            let gap = 45 + self.rng.below(55);
            self.spawn_in = (gap * BASE_SPEED as u32) / speed as u32 + 15;
        }
        if self.item_in > 0 {
            self.item_in -= 1;
        }
        if self.item_in == 0 {
            self.spawn_item();
            self.item_in = 132 + self.rng.below(193);
        }

        // Move world
        for o in self.obstacles.iter_mut().filter(|o| o.active) {
            o.x -= speed;
            if o.kind == KIND_BAT {
                o.phase += 1;
                o.y = o.base_y + triangle(o.phase, 72, 20) * FP;
            }
            if o.x + o.w * FP < -60 * FP {
                o.active = false;
            }
        }
        for it in self.items.iter_mut().filter(|i| i.active) {
            it.x -= speed;
            if it.x + ITEM_SIZE * FP < -40 * FP {
                it.active = false;
            }
        }
        for f in self.fireballs.iter_mut().filter(|f| f.active) {
            f.x += FIREBALL_SPEED;
            if f.x > (WORLD_W + 40) * FP {
                f.active = false;
            }
        }

        // Item pickups
        let hitbox = self.player_hitbox();
        let mut picked: [Option<Event>; MAX_ITEMS] = [None; MAX_ITEMS];
        for (idx, it) in self.items.iter_mut().enumerate().filter(|(_, i)| i.active) {
            let box_ = Aabb {
                x: it.x,
                y: it.y,
                w: ITEM_SIZE * FP,
                h: ITEM_SIZE * FP,
            };
            if overlap(&hitbox, &box_) {
                it.active = false;
                picked[idx] = Some(match it.kind {
                    k if k == ITEM_CHILI => Event::ChiliPepper,
                    k if k == ITEM_CAPE => Event::TableClothCape,
                    _ => Event::Espresso,
                });
            }
        }
        for event in picked.into_iter().flatten() {
            self.pickups += 1;
            self.apply_event(event);
        }

        // Fireball vs obstacle
        for fi in 0..MAX_FIREBALLS {
            if !self.fireballs[fi].active {
                continue;
            }
            let fbox = Aabb {
                x: self.fireballs[fi].x,
                y: self.fireballs[fi].y,
                w: FIREBALL_SIZE * FP,
                h: FIREBALL_SIZE * FP,
            };
            for oi in 0..MAX_OBSTACLES {
                if !self.obstacles[oi].active {
                    continue;
                }
                if overlap(&fbox, &Self::obstacle_hitbox(&self.obstacles[oi])) {
                    self.obstacles[oi].active = false;
                    self.fireballs[fi].active = false;
                    self.kills += 1;
                    break;
                }
            }
        }

        // Player vs obstacle
        for oi in 0..MAX_OBSTACLES {
            if !self.obstacles[oi].active {
                continue;
            }
            if overlap(
                &self.player_hitbox(),
                &Self::obstacle_hitbox(&self.obstacles[oi]),
            ) {
                self.hit_player();
                if self.over {
                    return;
                }
                self.obstacles[oi].active = false;
            }
        }
    }

    // --- State accessors (for rendering and journals) ---

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

    pub fn pickups(&self) -> u32 {
        self.pickups
    }

    pub fn kills(&self) -> u32 {
        self.kills
    }

    /// Distance scrolled, in whole pixels.
    pub fn distance_px(&self) -> u64 {
        (self.distance / i64::from(FP)) as u64
    }

    pub fn invulnerable(&self) -> bool {
        self.invuln > 0
    }

    pub fn player_px(&self) -> (i32, i32, i32, i32) {
        (PLAYER_X, self.player_y / FP, PLAYER_W, PLAYER_H)
    }

    pub fn grounded(&self) -> bool {
        self.grounded
    }

    /// Writes a flat render snapshot into `out`, returning the number of
    /// `i32` values written. Layout: repeated records of
    /// `[entity_type, kind, x_px, y_px, w_px, h_px]` where entity_type is
    /// 0 = obstacle (y = ground anchor for barrel/pipe, top edge for bat),
    /// 1 = item, 2 = fireball.
    pub fn snapshot(&self, out: &mut [i32]) -> usize {
        let mut n = 0;
        let mut push = |out: &mut [i32], rec: [i32; 6]| {
            if n + 6 <= out.len() {
                out[n..n + 6].copy_from_slice(&rec);
                n += 6;
            }
        };
        for o in self.obstacles.iter().filter(|o| o.active) {
            push(out, [0, o.kind, o.x / FP, o.y / FP, o.w, o.h]);
        }
        for it in self.items.iter().filter(|i| i.active) {
            push(
                out,
                [1, it.kind, it.x / FP, it.y / FP, ITEM_SIZE, ITEM_SIZE],
            );
        }
        for f in self.fireballs.iter().filter(|f| f.active) {
            push(
                out,
                [2, 0, f.x / FP, f.y / FP, FIREBALL_SIZE, FIREBALL_SIZE],
            );
        }
        n
    }

    /// Maximum `snapshot` output length in i32s.
    pub const SNAPSHOT_CAP: usize = (MAX_OBSTACLES + MAX_ITEMS + MAX_FIREBALLS) * 6;
}

/// Replays a full input trace (one byte per tick) from the given seed.
///
/// The run is considered finished when the sim reports game over or the
/// trace/`MAX_TICKS` is exhausted. Returns the resulting score and tick
/// count.
pub fn replay(seed: u64, trace: &[u8]) -> RunResult {
    let mut sim = Sim::new(seed);
    for &input in trace.iter().take(MAX_TICKS as usize) {
        if sim.over() {
            break;
        }
        sim.tick(input);
    }
    RunResult {
        score: sim.score(),
        ticks: sim.ticks(),
        over: sim.over(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_replay() {
        let mut trace = [0u8; 2000];
        // Pseudo-input: hop periodically.
        for (i, byte) in trace.iter_mut().enumerate() {
            if i % 50 < 12 {
                *byte = INPUT_JUMP;
            }
        }
        let a = replay(42, &trace);
        let b = replay(42, &trace);
        assert_eq!(a, b);
        let c = replay(43, &trace);
        assert_ne!(a, c);
    }

    #[test]
    fn idle_run_dies() {
        // Never jumping must eventually end the run.
        let trace = [0u8; 30_000];
        let result = replay(7, &trace);
        assert!(result.over);
        assert!(result.ticks < 30_000);
    }

    #[test]
    fn score_grows_with_survival() {
        let mut sim = Sim::new(1);
        for _ in 0..600 {
            if sim.over() {
                break;
            }
            sim.tick(0);
        }
        assert!(sim.score() > 0 || sim.over());
    }

    #[test]
    fn max_ticks_bounds_run() {
        let mut sim = Sim::new(5);
        let mut safety = 0u64;
        while !sim.over() && safety < u64::from(MAX_TICKS) + 10 {
            // Perfect-ish play: alternate jumps to survive a while; the cap
            // must end the run regardless.
            sim.tick(if safety % 40 < 10 { INPUT_JUMP } else { 0 });
            safety += 1;
        }
        assert!(sim.ticks() <= MAX_TICKS);
    }
}
