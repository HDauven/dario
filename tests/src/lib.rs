#[cfg(test)]
mod tests {
    #[macro_export]
    macro_rules! contract_bytecode {
        ($name:literal) => {
            include_bytes!(concat!("../../target/stripped/", $name, ".wasm"))
        };
    }

    use dusk_bytes::Serializable;
    use dusk_core::abi::ContractId;
    use dusk_core::abi::Metadata;
    use dusk_core::signatures::bls::{PublicKey, SecretKey};
    use dusk_core::transfer::TRANSFER_CONTRACT;
    use dusk_vm::{ContractData, Error, Session, VM};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    const OWNER: [u8; 32] = [0u8; 32];
    const LIMIT: u64 = 1_000_000_000;

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

    fn setup_with_moonlight_router() -> Result<(Session, ContractId), Error> {
        let vm = VM::ephemeral()?;
        let mut session = VM::genesis_session(&vm, 1);

        let dario_id = session.deploy(
            contract_bytecode!("contract"),
            ContractData::builder().owner(OWNER),
            LIMIT,
        )?;

        session.deploy(
            contract_bytecode!("moonlight_router"),
            ContractData::builder()
                .owner(OWNER)
                .contract_id(TRANSFER_CONTRACT),
            LIMIT,
        )?;

        Ok((session, dario_id))
    }

    fn moonlight_account(seed: u64) -> PublicKey {
        let mut rng = StdRng::seed_from_u64(seed);
        PublicKey::from(&SecretKey::random(&mut rng))
    }

    fn account_string(account: &PublicKey) -> String {
        bs58::encode(account.to_bytes()).into_string()
    }

    fn with_public_sender(session: &mut Session, sender: PublicKey) -> Result<(), Error> {
        session.set_meta(Metadata::PUBLIC_SENDER, Some(sender))?;
        Ok(())
    }

    fn routed_handle_event(
        session: &mut Session,
        sender: PublicKey,
        dario_id: ContractId,
        event: u32,
    ) -> Result<(), Error> {
        with_public_sender(session, sender)?;
        session.call::<_, ()>(TRANSFER_CONTRACT, "handle_event", &(dario_id, event), LIMIT)?;
        Ok(())
    }

    fn routed_current_state(
        session: &mut Session,
        sender: PublicKey,
        dario_id: ContractId,
    ) -> Result<u32, Error> {
        with_public_sender(session, sender)?;
        Ok(session
            .call::<_, u32>(TRANSFER_CONTRACT, "current_state", &dario_id, LIMIT)?
            .data)
    }

    fn routed_revive_count(
        session: &mut Session,
        sender: PublicKey,
        dario_id: ContractId,
    ) -> Result<u32, Error> {
        with_public_sender(session, sender)?;
        Ok(session
            .call::<_, u32>(TRANSFER_CONTRACT, "revive_count", &dario_id, LIMIT)?
            .data)
    }

    macro_rules! assert_state_event {
        ($receipt:expr, $expected_state:expr) => {{
            // Check that there indeed is one event emitted
            assert_eq!($receipt.events.len(), 1);
            // Check that the event emitted has the topic "state"
            assert_eq!($receipt.events[0].topic, "state");
            // Check that Dario's state matches (u32 LE bytes)
            assert_eq!(
                $receipt.events[0].data,
                ($expected_state as u32).to_le_bytes()
            );
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

        // Fire -> Cape (TableClothCape)
        let receipt = session.call::<_, ()>(dario_id, "handle_event", &TABLE_CLOTH_CAPE, LIMIT)?;
        assert_state_event!(receipt, 3);

        // Cape -> Fire (ChiliPepper)
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

    #[test]
    pub fn test_moonlight_accounts_have_isolated_state() -> Result<(), Error> {
        let (mut session, dario_id) = setup_with_moonlight_router()?;
        let wallet_a = moonlight_account(1);
        let wallet_b = moonlight_account(2);

        routed_handle_event(&mut session, wallet_a, dario_id, ESPRESSO)?;
        routed_handle_event(&mut session, wallet_b, dario_id, CHILI_PEPPER)?;

        assert_eq!(routed_current_state(&mut session, wallet_a, dario_id)?, 1);
        assert_eq!(routed_current_state(&mut session, wallet_b, dario_id)?, 2);

        routed_handle_event(&mut session, wallet_a, dario_id, TAKE_DAMAGE)?;
        routed_handle_event(&mut session, wallet_a, dario_id, TAKE_DAMAGE)?;
        routed_handle_event(&mut session, wallet_a, dario_id, REVIVE)?;

        assert_eq!(routed_current_state(&mut session, wallet_a, dario_id)?, 0);
        assert_eq!(routed_revive_count(&mut session, wallet_a, dario_id)?, 1);

        assert_eq!(routed_current_state(&mut session, wallet_b, dario_id)?, 2);
        assert_eq!(routed_revive_count(&mut session, wallet_b, dario_id)?, 0);

        assert_eq!(
            session
                .call::<_, u32>(
                    dario_id,
                    "current_state_for",
                    &account_string(&wallet_a),
                    LIMIT
                )?
                .data,
            0
        );
        assert_eq!(
            session
                .call::<_, u32>(
                    dario_id,
                    "revive_count_for",
                    &account_string(&wallet_b),
                    LIMIT
                )?
                .data,
            0
        );

        Ok(())
    }
}
