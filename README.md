# Mario FSM

This repository contains a [Finite-State Machine](https://en.wikipedia.org/wiki/Finite-state_machine) (FSM) implementation of Mario in [Super Mario World](https://en.wikipedia.org/wiki/Super_Mario_World).

## Structure 

The project is organized in three main components:
- `contract`: The smart contract that utilizes the Mario FSM for state transitions. It contains the logic to read state and set state. Built to run on the [Dusk protocol](https://github.com/dusk-network).
- `mario_fsm`: The core Rust library implementing the Mario FSM containing the state transition logic, events and states.
- `test`: A test suite for the smart contract.

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