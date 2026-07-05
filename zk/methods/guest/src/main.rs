//! Dario Dash zkVM guest: replays a recorded input trace through the
//! deterministic simulation and commits the result.
//!
//! Input (via `env::read_frame`-style raw bytes, written by the prover):
//!   [0..96)    account   — compressed Moonlight BLS public key bytes
//!   [96..104)  seed      — u64 little-endian
//!   [104..108) trace_len — u32 little-endian
//!   [108..)    trace     — one input byte per tick
//!
//! Journal (committed, fixed layout):
//!   account (96) || seed (8, LE) || score (8, LE) || ticks (4, LE)

use risc0_zkvm::guest::env;

const ACCOUNT_LEN: usize = 96;

fn main() {
    let mut header = [0u8; ACCOUNT_LEN + 8 + 4];
    env::read_slice(&mut header);

    let mut account = [0u8; ACCOUNT_LEN];
    account.copy_from_slice(&header[..ACCOUNT_LEN]);
    let seed = u64::from_le_bytes(header[ACCOUNT_LEN..ACCOUNT_LEN + 8].try_into().unwrap());
    let trace_len =
        u32::from_le_bytes(header[ACCOUNT_LEN + 8..ACCOUNT_LEN + 12].try_into().unwrap());

    assert!(
        trace_len <= dash_core::MAX_TICKS,
        "trace exceeds maximum run length"
    );

    let mut trace = vec![0u8; trace_len as usize];
    env::read_slice(&mut trace);

    let result = dash_core::replay(seed, &trace);
    assert!(result.over, "run did not end in game over");

    let mut journal = Vec::with_capacity(ACCOUNT_LEN + 8 + 8 + 4);
    journal.extend_from_slice(&account);
    journal.extend_from_slice(&seed.to_le_bytes());
    journal.extend_from_slice(&result.score.to_le_bytes());
    journal.extend_from_slice(&result.ticks.to_le_bytes());
    env::commit_slice(&journal);
}
