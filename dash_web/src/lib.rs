//! wasm-bindgen wrapper around the deterministic `dash_core` sim.
//!
//! The browser runs the *exact same* simulation the RISC Zero guest replays,
//! recording the input trace so a finished run can be proven and submitted
//! on-chain.

use dash_core::Sim;
use dash_zk::ZkSim;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct DashSim {
    sim: Sim,
    trace: Vec<u8>,
}

#[wasm_bindgen]
impl DashSim {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> DashSim {
        DashSim {
            sim: Sim::new(seed),
            trace: Vec::new(),
        }
    }

    /// Advances one 60Hz tick, recording `input` (bit0 jump, bit1 fire).
    pub fn tick(&mut self, input: u8) {
        if self.sim.over() || self.trace.len() >= dash_core::MAX_TICKS as usize {
            return;
        }
        self.trace.push(input & dash_core::INPUT_MASK);
        self.sim.tick(input);
    }

    pub fn over(&self) -> bool {
        self.sim.over()
    }

    pub fn score(&self) -> u64 {
        self.sim.score()
    }

    pub fn ticks(&self) -> u32 {
        self.sim.ticks()
    }

    /// Current FSM form (DarioState as u32).
    pub fn form(&self) -> u32 {
        self.sim.form()
    }

    pub fn pickups(&self) -> u32 {
        self.sim.pickups()
    }

    pub fn kills(&self) -> u32 {
        self.sim.kills()
    }

    pub fn distance_px(&self) -> u64 {
        self.sim.distance_px()
    }

    pub fn invulnerable(&self) -> bool {
        self.sim.invulnerable()
    }

    pub fn grounded(&self) -> bool {
        self.sim.grounded()
    }

    /// Player rect in pixels: [x, y, w, h].
    pub fn player(&self) -> Vec<i32> {
        let (x, y, w, h) = self.sim.player_px();
        vec![x, y, w, h]
    }

    /// Flat entity records: [entity_type, kind, x, y, w, h] per entity.
    /// entity_type: 0 obstacle, 1 item, 2 fireball.
    pub fn snapshot(&self) -> Vec<i32> {
        let mut buf = [0i32; Sim::SNAPSHOT_CAP];
        let n = self.sim.snapshot(&mut buf);
        buf[..n].to_vec()
    }

    /// The recorded input trace (one byte per tick), for proving.
    pub fn trace(&self) -> Vec<u8> {
        self.trace.clone()
    }
}

/// wasm wrapper around the 30 Hz browser-provable `dash_zk` sim.
///
/// Same JS-facing API as [`DashSim`], plus [`ZkDashSim::input_json`], which
/// serializes the full circom witness input for snarkjs `fullProve` after a
/// finished run — everything needed to prove entirely in-browser.
#[wasm_bindgen]
pub struct ZkDashSim {
    sim: ZkSim,
    trace: Vec<u8>,
}

#[wasm_bindgen]
impl ZkDashSim {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> ZkDashSim {
        ZkDashSim {
            sim: ZkSim::new(seed),
            trace: Vec::new(),
        }
    }

    /// Advances one 30Hz tick, recording `input` (bit0 jump, bit1 fire).
    pub fn tick(&mut self, input: u8) {
        if self.sim.over() || self.sim.ticks() >= dash_zk::MAX_TICKS {
            return;
        }
        let input = input & (dash_zk::INPUT_JUMP | dash_zk::INPUT_FIRE);
        self.trace.push(input);
        self.sim.tick(input);
    }

    pub fn over(&self) -> bool {
        self.sim.over()
    }

    pub fn score(&self) -> u64 {
        self.sim.score()
    }

    pub fn ticks(&self) -> u32 {
        self.sim.ticks()
    }

    /// Current FSM form (DarioState as u32).
    pub fn form(&self) -> u32 {
        self.sim.form()
    }

    pub fn pickups(&self) -> u32 {
        self.sim.pickups_total()
    }

    pub fn kills(&self) -> u32 {
        self.sim.kills_total()
    }

    pub fn distance_px(&self) -> u64 {
        self.sim.distance_px()
    }

    pub fn invulnerable(&self) -> bool {
        self.sim.invulnerable()
    }

    pub fn grounded(&self) -> bool {
        self.sim.grounded()
    }

    /// Player rect in pixels: [x, y, w, h].
    pub fn player(&self) -> Vec<i32> {
        let (x, y, w, h) = self.sim.player_px();
        vec![x, y, w, h]
    }

    /// Flat entity records: [entity_type, kind, x, y, w, h] per entity.
    /// entity_type: 0 obstacle, 1 item, 2 fireball.
    pub fn snapshot(&self) -> Vec<i32> {
        let mut buf = [0i32; 80 * 6];
        let n = self.sim.snapshot(&mut buf);
        buf[..n].to_vec()
    }

    /// The recorded input trace (one byte per tick).
    pub fn trace(&self) -> Vec<u8> {
        self.trace.clone()
    }

    /// The full snarkjs circuit input (publics + private witness) for the
    /// finished run. `acct_hex` is the caller's 96-byte Moonlight account
    /// (192 hex chars), or empty to leave the account unbound (zeros).
    pub fn input_json(&self, acct_hex: &str) -> Result<String, JsError> {
        let mut acct = [0u128; 6];
        if !acct_hex.is_empty() {
            if acct_hex.len() != 192 {
                return Err(JsError::new("account hex must encode 96 bytes"));
            }
            let mut bytes = [0u8; 96];
            for (i, b) in bytes.iter_mut().enumerate() {
                *b = u8::from_str_radix(&acct_hex[i * 2..i * 2 + 2], 16)
                    .map_err(|_| JsError::new("invalid account hex"))?;
            }
            for (i, limb) in acct.iter_mut().enumerate() {
                let mut chunk = [0u8; 16];
                chunk.copy_from_slice(&bytes[i * 16..(i + 1) * 16]);
                *limb = u128::from_le_bytes(chunk);
            }
        }
        Ok(dash_zk::input_json::build_input_json(&self.sim, &acct))
    }
}
