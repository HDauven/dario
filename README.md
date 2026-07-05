# Dario FSM

This repository contains a [Finite-State Machine](https://en.wikipedia.org/wiki/Finite-state_machine) (FSM) implementation of Dario's life cycle.

## Structure 

The project is organized in these main components:
- `contract`: The smart contract that utilizes the Dario FSM for state transitions and verifies Groth16 gameplay proofs on-chain via Dusk's `verify_groth16_bn254` host function — both RISC Zero receipts and browser-generated snarkjs proofs. Built to run on the [Dusk protocol](https://github.com/dusk-network).
- `dario_fsm`: The core Rust library implementing the Dario FSM containing the state transition logic, events and states.
- `dash_core`: A `no_std`, deterministic, integer-only simulation of the Dario Dash endless runner at 60 Hz, used by the RISC Zero proving path.
- `dash_zk`: A `no_std`, 30 Hz variant of the sim whose physics are closed-form, so runs can be proven by a circom circuit **directly in the browser**. Also generates the obstacle schedule from the seed and extracts the ZK witness. The same code runs in the browser (wasm) and in the contract.
- `dash_web`: A thin wasm-bindgen wrapper exposing the sims to the web app.
- `zk`: A separate cargo workspace with the RISC Zero guest program (replays a recorded input trace) and the `dash-prover` CLI (proves runs, exports contract verification constants for both proving paths, and verifies browser proofs through a contract-equivalent pipeline).
- `zk_browser`: The circom circuit (`circuits/dash_zk.circom`, ~421k constraints) proving a full Dash run — jump parabolas, obstacle clearance windows, bat sine-hover collision, fireball kills, FSM form transitions, item pickups and score — plus the snarkjs → ark proof converter.
- `tests`: A test suite for the smart contract, including on-chain verification of real checked-in Groth16 proofs from both proving paths.
- `web`: A bundled playable Dario demo, ported from the `@dusk/connect`
  example and wired to this contract's generated data-driver.

## Prerequisites

- [Rustup](https://rustup.rs/)
- [Make](https://www.gnu.org/software/make/)

## Build and test

To build the contract, simply run:

```bash
make contract
```

To test, run:

```bash
make test
```

To build the data-driver WASM used by browser clients, run:

```bash
make data-driver
```

## Web app

The `web` directory contains **Dario Dash**, an endless-runner mini game built
with Vite. Gameplay runs entirely in the browser: Dario auto-runs, you jump
over obstacles and collect power-ups that drive the same FSM as the contract
(☕ Espresso → Super jump, 🌶️ Chili → fireballs, 🧣 Cape → glide). Getting hit
as Regular ends the run.

The app uses [`@dusk/connect`](https://github.com/dusk-network/connect) to
connect to Dusk Wallet and submit public Moonlight contract calls. The game
runs on the deterministic `dash_zk` simulation compiled to wasm, recording
your inputs (one byte per 30 Hz tick) as it plays. The game is free to play
without a wallet.

## ZK proof of gameplay — in the browser

A finished run can be **cryptographically proven and submitted without
leaving the browser**. Click **Prove In-Browser & Submit** after a run:

1. The app serializes the run's witness (jumps, pickups, kills, damage
   events and per-obstacle clearance data) plus the public inputs (score,
   ticks, the seed-derived obstacle schedule, and your account).
2. A Web Worker downloads the proving artifacts (circuit wasm + ~200 MB
   proving key, cached by the browser) and runs `snarkjs groth16.fullProve`
   over the `dash_zk` circuit — about 10–60 s depending on the machine.
3. The proof is converted to the 128-byte ark-0.4 compressed format and
   submitted via `submit_zk_run(seed, score, ticks, proof)`. The contract
   **recomputes the obstacle schedule from the seed natively** (the same
   `dash_zk` code), binds the *transaction sender* into the public inputs,
   and verifies the proof with `verify_groth16_bn254`. Only then is the
   score recorded.

Proving artifacts are built once (`circom` + `snarkjs` setup in
`zk_browser/`) and copied into the app with `make zk-assets`. To regenerate
the contract's verification key assets after a circuit change:

```bash
zk/target/release/dash-prover export-snarkjs-vkey \
  zk_browser/build/dash/vkey.json contract/assets dash_zk
```

## ZK proof of gameplay — RISC Zero (native)

The original proving path is still supported by the contract
(`submit_run`) for `dash_core` runs proven off-line:

1. Export a run file `{account, seed, input trace}` (60 Hz `dash_core` trace).
2. Prove it locally (x86 + Docker required for the Groth16 wrap):

   ```bash
   make prove RUN=dario-run-123.json OUT=bundle.json
   ```

   The RISC Zero guest replays your trace with `dash_core` and commits a
   journal binding your account, seed, score and tick count. The STARK is
   wrapped into a Groth16 proof and converted to the ark-0.4 format Dusk's
   `verify_groth16_bn254` host function expects.
3. Submit the bundle's proof via `submit_run`. The contract reconstructs
   the journal from the *transaction sender*, recomputes the RISC Zero
   claim digest, prepares the public inputs and verifies the proof
   on-chain. Only then is the score recorded.

The contract keeps per-account best scores and proven-run counts
(`best_score_for`, `proven_runs_for`, `leaderboard`) and rejects seed
replays. If the guest program changes, regenerate the verification
constants with `make zk-constants` and rebuild the contract.

Create `web/.env.local` with the deployed contract id:

```bash
VITE_DARIO_CONTRACT_ID=0x...
VITE_DUSK_NODE_URL=https://testnet.nodes.dusk.network
```

Then run:

```bash
npm --prefix web install
make web
```

For a production build:

```bash
make web-build
```

## GitHub Pages

The Pages workflow builds the contract data-driver and publishes `web/dist`.
Configure these repository variables before deploying:

- `DARIO_CONTRACT_ID`: the deployed Dario contract id on Testnet.
- `DUSK_NODE_URL`: optional, defaults to `https://testnet.nodes.dusk.network`.
