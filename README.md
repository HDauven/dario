# Dario FSM

This repository contains a [Finite-State Machine](https://en.wikipedia.org/wiki/Finite-state_machine) (FSM) implementation of Dario's life cycle.

## Structure 

The project is organized in these main components:
- `contract`: The smart contract that utilizes the Dario FSM for state transitions. It contains the logic to read state and set state. Built to run on the [Dusk protocol](https://github.com/dusk-network).
- `dario_fsm`: The core Rust library implementing the Dario FSM containing the state transition logic, events and states.
- `tests`: A test suite for the smart contract.
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

The `web` directory contains a Vite app that uses
[`@dusk/connect`](https://github.com/dusk-network/connect) to connect to Dusk
Wallet and submit public Moonlight contract calls.

The app reads the connected wallet profile from Connect and calls
`current_state_for(account)` / `revive_count_for(account)`, so the displayed
state matches the Moonlight account that submits transactions.

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
