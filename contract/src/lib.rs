//! # MDrio FSM contract
//!
//! This contract is a simple implementation of a Finite-State Machine (FSM) using the Dario FSM library.
//! It manages the state of Dario in a blockchain context, and provides a function to read the state and handle events.
#![no_std]

extern crate alloc;

// Import the Dario FSM library
use dario_fsm::{transition, DarioState, Event};
use piecrust_macros::contract;
// Import Piecrust functionality, primarily for making calls and emitting events
use piecrust_uplink as uplink;

/// The DarioFSM struct describes how the state for this contract looks like
pub struct DarioFSM {
    current_state: DarioState,
}

/// State of the Dario contract on deploy
static mut STATE: DarioFSM = DarioFSM {
    current_state: DarioState::Regular,
};

#[contract]
impl DarioFSM {
    /// Reads the current state of DarioFSM and returns it as an integer
    ///
    /// The state is returned as a `u32` corresponding to the `DarioState` enum
    pub fn read_state(&self) -> u32 {
        self.current_state as u32
    }

    /// Handles a game event and updates Dario's state accordingly
    ///
    /// # Arguments
    ///
    /// * `Event` - A `u32` corresponding to the `Event` enum
    pub fn handle_event(&mut self, event: u32) {
        // TakeDamage is the highest value supported by the enum,
        // we should not accept a higher value
        if event > Event::TakeDamage as u32 {
            panic!("Invalid event number passed");
        }
        // This transmute should be safe given we check for the upper bound of `Event`
        let event = unsafe { core::mem::transmute::<u32, Event>(event) };
        // Call the Dario FSM transition function to get a new state, if applicable
        let new_state = transition(self.current_state, event);
        // Set the contract state to the new state
        self.current_state = new_state;
        // Emit a "state" event to inform blockchain listeners of Dario's state after this transaction
        // We could also choose to not emit an event if Dario's state did not change
        uplink::emit("state", self.current_state as u32);
    }
}
