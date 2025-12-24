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

    // Event ids (must match the contract)
    const ESPRESSO: u32 = 0;
    const CHILI_PEPPER: u32 = 1;
    const TABLE_CLOTH_CAPE: u32 = 2;
    const TAKE_DAMAGE: u32 = 3;
    const REVIVE: u32 = 4;

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

    macro_rules! assert_state_event {
        ($receipt:expr, $expected_state:expr) => {{
            // Check that there indeed is one event emitted
            assert_eq!($receipt.events.len(), 1);
            // Check that the event emitted has the topic "state"
            assert_eq!($receipt.events[0].topic, "state");
            // Check that Dario's state matches (u32 LE bytes)
            assert_eq!($receipt.events[0].data, ($expected_state as u32).to_le_bytes());
        }};
    }

    #[test]
    pub fn test_dario_fsm_transitions_and_events() -> Result<(), Error> {
        let (mut session, dario_id) = setup()?;

        // Regular -> Super (Espresso)
        let receipt = session.call::<_, ()>(dario_id, "handle_event", &ESPRESSO, LIMIT)?;
        assert_state_event!(receipt, 1);

        // Super -> Fire (ChiliPepper)
        let receipt = session.call::<_, ()>(dario_id, "handle_event", &CHILI_PEPPER, LIMIT)?;
        assert_state_event!(receipt, 2);

        // Fire -> Regular (TakeDamage)
        let receipt = session.call::<_, ()>(dario_id, "handle_event", &TAKE_DAMAGE, LIMIT)?;
        assert_state_event!(receipt, 0);

        // Regular -> GameOver (TakeDamage)
        let receipt = session.call::<_, ()>(dario_id, "handle_event", &TAKE_DAMAGE, LIMIT)?;
        assert_state_event!(receipt, 4);

        // While GameOver, other events keep him GameOver
        let receipt = session.call::<_, ()>(dario_id, "handle_event", &ESPRESSO, LIMIT)?;
        assert_state_event!(receipt, 4);

        // GameOver -> Regular (Revive)
        let receipt = session.call::<_, ()>(dario_id, "handle_event", &REVIVE, LIMIT)?;
        assert_state_event!(receipt, 0);

        Ok(())
    }

    #[test]
    pub fn test_invalid_event() -> Result<(), Error> {
        let (mut session, dario_id) = setup()?;

        // Valid event numbers are 0..=4. 5 should fail.
        let invalid_event: u32 = 5;
        let result = session.call::<_, ()>(dario_id, "handle_event", &invalid_event, LIMIT);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    pub fn test_current_state() -> Result<(), Error> {
        let (mut session, dario_id) = setup()?;

        // Initial state should be Regular
        let receipt = session.call::<_, u32>(dario_id, "current_state", &(), LIMIT)?;
        assert_eq!(receipt.data, 0);

        // Espresso -> Super
        session.call::<_, ()>(dario_id, "handle_event", &ESPRESSO, LIMIT)?;
        let receipt = session.call::<_, u32>(dario_id, "current_state", &(), LIMIT)?;
        assert_eq!(receipt.data, 1);

        // ChiliPepper -> Fire
        session.call::<_, ()>(dario_id, "handle_event", &CHILI_PEPPER, LIMIT)?;
        let receipt = session.call::<_, u32>(dario_id, "current_state", &(), LIMIT)?;
        assert_eq!(receipt.data, 2);

        Ok(())
    }

    #[test]
    pub fn test_revive_count_and_revive_rules() -> Result<(), Error> {
        let (mut session, dario_id) = setup()?;

        // Initial revive_count should be 0
        let receipt = session.call::<_, u32>(dario_id, "revive_count", &(), LIMIT)?;
        assert_eq!(receipt.data, 0);

        // Revive when NOT GameOver should do nothing and NOT increment revive_count
        session.call::<_, ()>(dario_id, "handle_event", &REVIVE, LIMIT)?;
        let st = session.call::<_, u32>(dario_id, "current_state", &(), LIMIT)?;
        let rv = session.call::<_, u32>(dario_id, "revive_count", &(), LIMIT)?;
        assert_eq!(st.data, 0); // still Regular
        assert_eq!(rv.data, 0); // no increment

        // Finish Dario: Regular -> GameOver
        session.call::<_, ()>(dario_id, "handle_event", &TAKE_DAMAGE, LIMIT)?;
        let st = session.call::<_, u32>(dario_id, "current_state", &(), LIMIT)?;
        assert_eq!(st.data, 4);

        // Revive: GameOver -> Regular and revive_count increments
        session.call::<_, ()>(dario_id, "handle_event", &REVIVE, LIMIT)?;
        let st = session.call::<_, u32>(dario_id, "current_state", &(), LIMIT)?;
        let rv = session.call::<_, u32>(dario_id, "revive_count", &(), LIMIT)?;
        assert_eq!(st.data, 0);
        assert_eq!(rv.data, 1);

        // Finish + revive again => revive_count becomes 2
        session.call::<_, ()>(dario_id, "handle_event", &TAKE_DAMAGE, LIMIT)?;
        session.call::<_, ()>(dario_id, "handle_event", &REVIVE, LIMIT)?;
        let rv = session.call::<_, u32>(dario_id, "revive_count", &(), LIMIT)?;
        assert_eq!(rv.data, 2);

        Ok(())
    }
}
