# Dario FSM

This repository contains a [Finite-State Machine](https://en.wikipedia.org/wiki/Finite-state_machine) (FSM) implementation of Dario's life cycle.

## Structure 

The project is organized in three main components:
- `contract`: The smart contract that utilizes the Dario FSM for state transitions. It contains the logic to read state and set state. Built to run on the [Dusk protocol](https://github.com/dusk-network).
- `dario_fsm`: The core Rust library implementing the Dario FSM containing the state transition logic, events and states.
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