//! Exports a snarkjs `input.json` for the dash_zk circom circuit from a
//! deterministic autopilot run (or a replayed input trace).
//!
//! Usage: export_input <seed> <out_input.json> [trace_file|-] [acct_hex_96B]
//!
//! The trace file, if given, is raw bytes, one input byte per tick
//! (bit0 jump, bit1 fire). Without it (or with `-`) a built-in autopilot
//! plays. The optional account hex (192 chars) is bound into the proof as
//! 6 little-endian u128 limbs.

use dash_zk::input_json::build_input_json;
use dash_zk::*;

fn autopilot_run(seed: u64) -> ZkSim {
    let mut sim = ZkSim::new(seed);
    let mut snap = [0i32; 40 * 6];
    let mut prev = 0u8;
    let mut t = 0u32;
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
        let mut input = if want && prev & INPUT_JUMP == 0 {
            INPUT_JUMP
        } else {
            0
        };
        // Fire on a rising edge every other tick (no-op unless in Fire form).
        if t % 2 == 0 {
            input |= INPUT_FIRE;
        }
        prev = input;
        sim.tick(input);
        t += 1;
    }
    sim
}

fn replay_run(seed: u64, trace: &[u8]) -> ZkSim {
    let mut sim = ZkSim::new(seed);
    for &b in trace {
        if sim.over() || sim.ticks() >= MAX_TICKS {
            break;
        }
        sim.tick(b);
    }
    sim
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: export_input <seed> <out_input.json> [trace_file|-] [acct_hex_96B]");
        std::process::exit(1);
    }
    let seed: u64 = args[1].parse().expect("seed");
    let sim = if args.len() > 3 && args[3] != "-" {
        let trace = std::fs::read(&args[3]).expect("trace file");
        replay_run(seed, &trace)
    } else {
        autopilot_run(seed)
    };
    // Account bound into the proof: 96 bytes as 6 little-endian u128 limbs.
    let acct: [u128; 6] = match args.get(4) {
        Some(hex) => {
            assert_eq!(hex.len(), 192, "acct hex must encode 96 bytes");
            let bytes: Vec<u8> = (0..96)
                .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("acct hex"))
                .collect();
            core::array::from_fn(|i| {
                u128::from_le_bytes(bytes[i * 16..(i + 1) * 16].try_into().unwrap())
            })
        }
        None => [0; 6],
    };

    eprintln!(
        "seed={} ticks={} score={} over={} jumps={} pickups={} kills={} damages={}",
        seed,
        sim.ticks(),
        sim.score(),
        sim.over(),
        sim.jump_count,
        sim.pickup_count,
        sim.kill_count,
        sim.damage_count,
    );

    std::fs::write(&args[2], build_input_json(&sim, &acct)).expect("write output");
    eprintln!("wrote {}", args[2]);
}
