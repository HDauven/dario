//! # Dario FSM lib
//!
//! The `dario_fsm` library demonstrates how a simple Finite-State Machine (FSM) implementation works, in this case specifically for managing the various states Dario can be in.
//! It allows for the simulation of Dario's state based on a given game event, like collecting a power-up or taking damage.
//!
//!  The library revolves around two enums: `DarioState`, representing Dario's possible states, and
//! `Event`, representing different events that can trigger state changes. The core functionality is encapsulated
//! in the `transition` function, which computes the new state of Dario based on the current state and an event.
#![no_std]

/// Represents the various states that Dario can be in.
///
/// # Examples
///
/// ```
/// use dario_fsm::DarioState;
///
/// let state = DarioState::Regular;
/// println!("{:?}", state); // Output: Regular
/// ```
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DarioState {
    Regular,
    Super,
    Fire,
    Cape,
    GameOver,
}

/// Represents the different events that can cause state transitions for Dario.
///
/// # Examples
///
/// ```
/// use dario_fsm::Event;
///
/// let event = Event::Espresso;
/// println!("{:?}", event); // Output: Espresso
/// ```
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Espresso,
    ChiliPepper,
    TableClothCape,
    TakeDamage,
    Revive,
}

/// Transitions Dario's state based on the provided event.
///
/// # Arguments
///
/// * `state` - The current state of Dario.
/// * `event` - The event causing the state transition.
///
/// # Examples
///
/// ```
/// use dario_fsm::{transition, DarioState, Event};
///
/// let current_state = DarioState::Regular;
/// let new_state = transition(current_state, Event::Espresso);
/// assert_eq!(new_state, DarioState::Super);
///
/// let new_state = transition(current_state, Event::ChiliPepper);
/// assert_eq!(new_state, DarioState::Fire);
///
/// ```
pub fn transition(state: DarioState, event: Event) -> DarioState {
    use DarioState::*;
    use Event::*;

    match (state, event) {
        (Regular, Espresso) => Super,
        (Regular, ChiliPepper) => Fire,
        (Regular, TableClothCape) => Cape,
        (Super, ChiliPepper) => Fire,
        (Super, TableClothCape) => Cape,
        (Fire, TableClothCape) => Cape,
        (Cape, ChiliPepper) => Fire,
        (Regular, TakeDamage) => GameOver,
        (Super, TakeDamage) | (Fire, TakeDamage) | (Cape, TakeDamage) => Regular,
        (GameOver, Revive) => Regular,
        (current_state, _) => current_state,
    }
}
