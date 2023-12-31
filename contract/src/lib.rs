//! # Mario FSM contract
//!
//! This contract is a simple implementation of a Finite-State Machine (FSM) using the Mario FSM library.
//! It manages the state of Mario in a blockchain context, and provides a function to read the state and handle events.
#![no_std]

extern crate alloc;

// Import the Mario FSM library
use mario_fsm::{transition, Event, MarioState};
// Import Piecrust functionality, primarily for making calls and emitting events
use piecrust_uplink as uplink;

/// The MarioFSM struct describes how the state for this contract looks like
pub struct MarioFSM {
    current_state: MarioState,
}

/// State of the Mario contract on deploy
static mut STATE: MarioFSM = MarioFSM {
    current_state: MarioState::Regular,
};

impl MarioFSM {
    /// Reads the current state of MarioFSM and returns it as an integer
    ///
    /// The state is returned as a `u32` corresponding to the `MarioState` enum
    pub fn read_state(&self) -> u32 {
        self.current_state as u32
    }

    /// Handles a game event and updates Mario's state accordingly
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
        // Call the Mario FSM transition function to get a new state, if applicable
        let new_state = transition(self.current_state, event);
        // Set the contract state to the new state
        self.current_state = new_state;
        // Emit a "state" event to inform blockchain listeners of Mario's state after this transaction
        // We could also choose to not emit an event if Mario's state did not change
        uplink::emit("state", self.current_state as u32);
    }
}

/// Expose `read_state` to external callers
#[no_mangle]
unsafe fn read_state(arg_len: u32) -> u32 {
    uplink::wrap_call(arg_len, |_: ()| STATE.read_state())
}

/// Expose `handle_event` to external callers
#[no_mangle]
unsafe fn handle_event(arg_len: u32) -> u32 {
    uplink::wrap_call(arg_len, |event: u32| STATE.handle_event(event))
}
