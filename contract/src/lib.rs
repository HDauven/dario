//! # Dario FSM contract
//!
//! This contract is a simple implementation of a Finite-State Machine (FSM) using the Dario FSM library.
//! It manages the state of Dario in a blockchain context, and provides a function to read the state and handle events.
//!
//! This contract uses the `#[contract]` macro from `dusk-wasm` to auto-generate
//! extern wrappers, schema, and data-driver implementations.
#![no_std]

extern crate alloc;

use dusk_forge::contract;

/// The DarioFSM contract module.
///
/// The `#[contract]` macro generates:
/// - Static `STATE` variable with the contract struct
/// - Extern "C" wrapper functions for WASM export
/// - `CONTRACT_SCHEMA` constant with metadata
/// - `data_driver` module when compiled with the `data-driver` feature
#[contract]
mod dario_fsm_contract {
    use alloc::collections::BTreeMap;
    use alloc::string::String;

    // Import the Dario FSM library
    use dario_fsm::{transition, DarioState, Event};
    // Import Dusk Core functionality, primarily for making calls and emitting events
    use dusk_bytes::Serializable;
    use dusk_core::signatures::bls::PublicKey;
    use dusk_core::transfer::TRANSFER_CONTRACT;
    use dusk_core::{self, abi};

    const ACCOUNT_KEY_BYTES: usize = 193;
    type AccountKey = [u8; ACCOUNT_KEY_BYTES];

    #[derive(Clone, Copy)]
    struct PlayerState {
        current_state: DarioState,
        revive_count: u32,
    }

    impl PlayerState {
        const fn new() -> Self {
            Self {
                current_state: DarioState::Regular,
                revive_count: 0,
            }
        }

        fn current_state(&self) -> u32 {
            self.current_state as u32
        }

        fn handle_event(&mut self, event: Event) {
            let previous_state = self.current_state;
            let new_state = transition(self.current_state, event);

            if previous_state == DarioState::GameOver && event == Event::Revive {
                self.revive_count = self.revive_count.saturating_add(1);
            }

            self.current_state = new_state;
        }
    }

    /// The DarioFSM struct describes how the state for this contract looks like
    /// There should only be one public struct
    pub struct DarioFSM {
        current_state: DarioState,
        revive_count: u32,
        players: BTreeMap<AccountKey, PlayerState>,
    }

    impl DarioFSM {
        /// Creates a new DarioFSM instance with initial state Regular.
        pub const fn new() -> Self {
            Self {
                current_state: DarioState::Regular,
                revive_count: 0,
                players: BTreeMap::new(),
            }
        }

        /// Returns the current caller-scoped state as a u32.
        ///
        /// Moonlight calls routed through the transfer contract read the
        /// public sender's state. Direct VM calls retain the original global
        /// state so local tests and simple read-only calls remain compatible.
        pub fn current_state(&self) -> u32 {
            if let Some(account) = moonlight_sender() {
                return self
                    .players
                    .get(&account)
                    .map(PlayerState::current_state)
                    .unwrap_or(DarioState::Regular as u32);
            }

            self.current_state as u32
        }

        /// Returns the current caller-scoped revive count.
        pub fn revive_count(&self) -> u32 {
            if let Some(account) = moonlight_sender() {
                return self
                    .players
                    .get(&account)
                    .map(|state| state.revive_count)
                    .unwrap_or(0);
            }

            self.revive_count
        }

        /// Returns a Moonlight public account's current Dario state.
        pub fn current_state_for(&self, account: String) -> u32 {
            let account = account_key_from_address(account);

            self.players
                .get(&account)
                .map(PlayerState::current_state)
                .unwrap_or(DarioState::Regular as u32)
        }

        /// Returns a Moonlight public account's revive count.
        pub fn revive_count_for(&self, account: String) -> u32 {
            let account = account_key_from_address(account);

            self.players
                .get(&account)
                .map(|state| state.revive_count)
                .unwrap_or(0)
        }

        /// Handles a game event and updates Dario's state accordingly
        ///
        /// # Arguments
        ///
        /// * `Event` - A `u32` corresponding to the `Event` enum
        pub fn handle_event(&mut self, event: u32) {
            let event = Event::try_from(event).expect("Invalid event number passed");

            if let Some(account) = moonlight_sender() {
                let state = self.players.entry(account).or_insert_with(PlayerState::new);
                state.handle_event(event);
                dusk_core::abi::emit("state", state.current_state as u32);
                return;
            }

            let previous_state = self.current_state;
            let new_state = transition(self.current_state, event);

            if previous_state == DarioState::GameOver && event == Event::Revive {
                self.revive_count = self.revive_count.saturating_add(1);
            }

            self.current_state = new_state;
            dusk_core::abi::emit("state", self.current_state as u32);
        }
    }

    impl Default for DarioFSM {
        fn default() -> Self {
            Self::new()
        }
    }

    fn moonlight_sender() -> Option<AccountKey> {
        let caller = abi::caller();
        let callstack_len = abi::callstack().len();

        if matches!(caller, Some(TRANSFER_CONTRACT)) && callstack_len <= 1 {
            abi::public_sender().map(|pk| pk.to_raw_bytes())
        } else {
            None
        }
    }

    fn account_key_from_address(account: String) -> AccountKey {
        let bytes = bs58::decode(account)
            .into_vec()
            .expect("Invalid Moonlight account encoding");
        let bytes: [u8; PublicKey::SIZE] = bytes
            .try_into()
            .unwrap_or_else(|_| panic!("Moonlight account must be a public account"));
        let key = PublicKey::from_bytes(&bytes).expect("Invalid Moonlight public account");

        key.to_raw_bytes()
    }
}
