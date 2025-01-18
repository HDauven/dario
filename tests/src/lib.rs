#[cfg(test)]
mod tests {
    #[macro_export]
    macro_rules! contract_bytecode {
        ($name:literal) => {
            include_bytes!(concat!("../../target/stripped/", $name, ".wasm"))
        };
    }

    use dusk_core::abi::ContractId;
    use dusk_vm::{ContractData, Error, Session, VM};

    const OWNER: [u8; 32] = [0u8; 32];
    const LIMIT: u64 = 1_000_000;

    // Basic setup function that deals with VM instantiation, session setup and contract deployment
    fn setup() -> Result<(Session, ContractId), Error> {
        let vm = VM::ephemeral()?;
        let mut session = VM::genesis_session(&vm, 1);

        // Deploy the DarioFSM contract
        let dario_id = session.deploy(
            contract_bytecode!("contract"),
            ContractData::builder().owner(OWNER),
            LIMIT,
        )?;

        Ok((session, dario_id))
    }

    #[test]
    pub fn test_dario_fsm() -> Result<(), Error> {
        let Ok((mut session, dario_id)) = setup() else {
            panic!("Setup failed")
        };

        // Test case: Regular -> Super (Salmon event)
        let receipt = session.call::<_, ()>(dario_id, "handle_event", &(0), LIMIT)?;
        // Check that there indeed is one event emitted
        assert_eq!(receipt.events.len(), 1);
        // Check that the event emitted has the topic "state"
        assert_eq!(receipt.events[0].topic, "state");
        // Check that Dario's state is 1 (Super)
        assert_eq!(receipt.events[0].data, 1_u32.to_le_bytes());

        // Test case: Super -> Fire (ChiliPepper event)
        let receipt = session.call::<_, ()>(dario_id, "handle_event", &(1), LIMIT)?;
        assert_eq!(receipt.events.len(), 1);
        assert_eq!(receipt.events[0].topic, "state");
        assert_eq!(receipt.events[0].data, 2_u32.to_le_bytes());

        Ok(())
    }

    #[test]
    pub fn test_invalid_event() -> Result<(), Error> {
        let Ok((mut session, dario_id)) = setup() else {
            panic!("Setup failed")
        };

        // Test case: Invalid event number leads to a panic
        // Use an invalid event number
        let result = session.call::<_, ()>(dario_id, "handle_event", &(5), LIMIT);
        // Check if the call resulted in an error
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    pub fn test_read_state() -> Result<(), Error> {
        let Ok((mut session, dario_id)) = setup() else {
            panic!("Setup failed")
        };

        // Initial state should be Regular
        let receipt = session.call::<_, u32>(dario_id, "read_state", &(), LIMIT)?;
        assert_eq!(receipt.data, 0_u32);

        // Test state after Espresso event
        session.call::<_, ()>(dario_id, "handle_event", &(0), LIMIT)?;
        let receipt = session.call::<_, u32>(dario_id, "read_state", &(), LIMIT)?;
        assert_eq!(receipt.data, 1_u32); // Super state

        // Test state after ChiliPepper event
        session.call::<_, ()>(dario_id, "handle_event", &(1), LIMIT)?;
        let receipt = session.call::<_, u32>(dario_id, "read_state", &(), LIMIT)?;
        assert_eq!(receipt.data, 2_u32); // Fire state

        Ok(())
    }
}
