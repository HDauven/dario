//! Test-only Moonlight router for local VM checks.
#![no_std]

use dusk_forge::contract;

#[contract]
mod moonlight_router {
    extern crate alloc;
    use alloc::vec::Vec;

    use dusk_core::abi::{self, ContractId};

    pub struct MoonlightRouter;

    impl MoonlightRouter {
        pub const fn new() -> Self {
            Self
        }

        pub fn handle_event(&mut self, args: (ContractId, u32)) {
            abi::call::<_, ()>(args.0, "handle_event", &args.1)
                .unwrap_or_else(|err| panic!("MoonlightRouter: {err:?}"));
        }

        pub fn current_state(&self, contract: ContractId) -> u32 {
            abi::call::<_, u32>(contract, "current_state", &())
                .unwrap_or_else(|err| panic!("MoonlightRouter: {err:?}"))
        }

        pub fn revive_count(&self, contract: ContractId) -> u32 {
            abi::call::<_, u32>(contract, "revive_count", &())
                .unwrap_or_else(|err| panic!("MoonlightRouter: {err:?}"))
        }

        pub fn submit_run(&mut self, args: (ContractId, u64, u64, u32, Vec<u8>)) {
            let (contract, seed, score, ticks, proof) = args;
            abi::call::<_, ()>(contract, "submit_run", &(seed, score, ticks, proof))
                .unwrap_or_else(|err| panic!("MoonlightRouter: {err:?}"));
        }

        pub fn submit_zk_run(&mut self, args: (ContractId, u64, u64, u32, Vec<u8>)) {
            let (contract, seed, score, ticks, proof) = args;
            abi::call::<_, ()>(contract, "submit_zk_run", &(seed, score, ticks, proof))
                .unwrap_or_else(|err| panic!("MoonlightRouter: {err:?}"));
        }
    }

    impl Default for MoonlightRouter {
        fn default() -> Self {
            Self::new()
        }
    }
}
