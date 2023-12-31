//! # Mario FSM lib
//!
//! The `mario_fsm` library demonstrates how a simple Finite-State Machine (FSM) implementation works, in this case specifically for managing the various states Mario can be in in Super Mario.
//! It allows for the simulation of Mario's state based on a given game event, like collecting a power-up or taking damage.
//!
//!  The library revolves around two enums: `MarioState`, representing Mario's possible states, and
//! `Event`, representing different events that can trigger state changes. The core functionality is encapsulated
//! in the `transition` function, which computes the new state of Mario based on the current state and an event.
#![no_std]

/// Represents the various states that Mario can be in.
///
/// # Examples
///
/// ```
/// use mario_fsm::MarioState;
///
/// let state = MarioState::Regular;
/// println!("{:?}", state); // Output: Regular
/// ```
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MarioState {
    Regular,
    Super,
    Fire,
    Cape,
    GameOver,
}

/// Represents the different events that can cause state transitions for Mario.
///
/// # Examples
///
/// ```
/// use mario_fsm::Event;
///
/// let event = Event::Mushroom;
/// println!("{:?}", event); // Output: Mushroom
/// ```
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Mushroom,
    FireFlower,
    CapeFeather,
    TakeDamage,
}

/// Transitions Mario's state based on the provided event.
///
/// # Arguments
///
/// * `state` - The current state of Mario.
/// * `event` - The event causing the state transition.
///
/// # Examples
///
/// ```
/// use mario_fsm::{transition, MarioState, Event};
///
/// let current_state = MarioState::Regular;
/// let new_state = transition(current_state, Event::Mushroom);
/// assert_eq!(new_state, MarioState::Super);
///
/// let new_state = transition(current_state, Event::FireFlower);
/// assert_eq!(new_state, MarioState::Fire);
///
/// ```
pub fn transition(state: MarioState, event: Event) -> MarioState {
    use Event::*;
    use MarioState::*;

    match (state, event) {
        (Regular, Mushroom) => Super,
        (Regular, FireFlower) => Fire,
        (Regular, CapeFeather) => Cape,
        (Super, FireFlower) => Fire,
        (Super, CapeFeather) => Cape,
        (Fire, CapeFeather) => Cape,
        (Cape, FireFlower) => Fire,
        (Regular, TakeDamage) => GameOver,
        (Super, TakeDamage) | (Fire, TakeDamage) | (Cape, TakeDamage) => Regular,
        (current_state, _) => current_state,
    }
}
