// Data driver for Dario FSM Contract
#![no_std]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

extern crate alloc;

use dusk_data_driver::{ConvertibleContract, Error, JsonValue, rkyv_to_json, json_to_rkyv};

/// Driver implementing JSON <-> RKYV for Dario FSM contract.
#[derive(Default)]
pub struct ContractDriver;

impl ConvertibleContract for ContractDriver {
    fn encode_input_fn(&self, fn_name: &str, json: &str) -> Result<alloc::vec::Vec<u8>, Error> {
        match fn_name {
            // Queries
            "read_state" => json_to_rkyv::<()>(&json),

            // Transactions
            "handle_event" => json_to_rkyv::<u32>(&json),

            // Unsupported
            name => Err(Error::Unsupported(alloc::format!("fn_name {name}"))),
        }
    }

    fn decode_input_fn(&self, fn_name: &str, rkyv: &[u8]) -> Result<JsonValue, Error> {
        match fn_name {
            // Queries
            "read_state" => rkyv_to_json::<()>(&rkyv),

            // Transactions
            "handle_event" => rkyv_to_json::<u32>(&rkyv),

            // Unsupported
            name => Err(Error::Unsupported(alloc::format!("fn_name {name}"))),
        }
    }

    fn decode_output_fn(&self, fn_name: &str, rkyv: &[u8]) -> Result<JsonValue, Error> {
        match fn_name {
            "read_state" => rkyv_to_json::<u32>(&rkyv),
            "handle_event" => rkyv_to_json::<()>(&rkyv),

            name => Err(Error::Unsupported(alloc::format!("fn_name {name}"))),
        }
    }

    fn decode_event(&self, event_name: &str, rkyv: &[u8]) -> Result<JsonValue, Error> {
        match event_name {
            "state" => rkyv_to_json::<u32>(&rkyv),

            event => Err(Error::Unsupported(alloc::format!("event {event}"))),
        }
    }

    fn get_schema(&self) -> alloc::string::String {
        alloc::string::String::from(include_str!("schema.json"))
    }
}

#[cfg(all(target_family = "wasm", feature = "ffi"))]
dusk_data_driver::generate_wasm_entrypoint!(ContractDriver);
