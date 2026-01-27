//! # Dario FSM contract
//!
//! This contract is a simple implementation of a Finite-State Machine (FSM) using the Dario FSM library.
//! It manages the state of Dario in a blockchain context, and provides a function to read the state and handle events.
//!
//! This contract uses the `#[contract]` macro from `dusk-wasm` to auto-generate
//! extern wrappers, schema, and data-driver implementations.
#![no_std]

use dusk_wasm::contract;

/// The DarioFSM contract module.
///
/// The `#[contract]` macro generates:
/// - Static `STATE` variable with the contract struct
/// - Extern "C" wrapper functions for WASM export
/// - `CONTRACT_SCHEMA` constant with metadata
/// - `data_driver` module when compiled with the `data-driver` feature
#[contract]
mod dario_fsm_contract {
    // Import the Dario FSM library
    use dario_fsm::{transition, DarioState, Event};
    // Import Dusk Core functionality, primarily for making calls and emitting events
    use dusk_core;

    /// The DarioFSM struct describes how the state for this contract looks like
    /// There should only be one public struct
    pub struct DarioFSM {
        current_state: DarioState,
        revive_count: u32,
    }

    impl DarioFSM {
        /// Creates a new DarioFSM instance with initial state Regular.
        pub const fn new() -> Self {
            Self {
                current_state: DarioState::Regular,
                revive_count: 0,
            }
        }

        /// Returns the current state of Dario as a u32.
        pub fn current_state(&self) -> u32 {
            self.current_state as u32
        }

        /// Returns the number of times Dario has been revived from GameOver.
        pub fn revive_count(&self) -> u32 {
            self.revive_count
        }

        /// Handles a game event and updates Dario's state accordingly
        ///
        /// # Arguments
        ///
        /// * `Event` - A `u32` corresponding to the `Event` enum
        pub fn handle_event(&mut self, event: u32) {
            // Revive is the highest value supported by the enum,
            // we should not accept a higher value
            if event > Event::Revive as u32 {
                panic!("Invalid event number passed");
            }
            // This transmute should be safe given we check for the upper bound of `Event`
            let event = unsafe { core::mem::transmute::<u32, Event>(event) };
            let previous_state = self.current_state;
            // Call the Dario FSM transition function to get a new state, if applicable
            let new_state = transition(self.current_state, event);

            // Count revivals: only when transitioning out of GameOver via Revive.
            if previous_state == DarioState::GameOver && event == Event::Revive {
                self.revive_count = self.revive_count.saturating_add(1);
            }

            // Set the contract state to the new state
            self.current_state = new_state;
            // Emit a "state" event to inform blockchain listeners of Dario's state after this transaction
            // We could also choose to not emit an event if Dario's state did not change
            dusk_core::abi::emit("state", self.current_state as u32);
        }
    }
}
