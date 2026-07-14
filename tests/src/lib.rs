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

    /// Helper for localnet E2E: prints the rkyv-encoded hex of the
    /// `submit_zk_run(seed, score, ticks, proof)` args tuple for use with
    /// `rusk-wallet contract-call --fn-args`. Run with:
    /// `ZK_SEED=42 ZK_SCORE=743 ZK_TICKS=997 ZK_PROOF_HEX_FILE=/tmp/e2e_proof.hex \
    ///  cargo test print_zk_call_args -- --ignored --nocapture`
    #[test]
    #[ignore]
    pub fn print_zk_call_args() {
        let seed: u64 = std::env::var("ZK_SEED").unwrap().parse().unwrap();
        let score: u64 = std::env::var("ZK_SCORE").unwrap().parse().unwrap();
        let ticks: u32 = std::env::var("ZK_TICKS").unwrap().parse().unwrap();
        let proof_hex =
            std::fs::read_to_string(std::env::var("ZK_PROOF_HEX_FILE").unwrap()).unwrap();
        let proof_hex = proof_hex.trim();
        let proof: Vec<u8> = (0..proof_hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&proof_hex[i..i + 2], 16).unwrap())
            .collect();
        let bytes = rkyv::to_bytes::<_, 4096>(&(seed, score, ticks, proof)).unwrap();
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        println!("{hex}");
    }

    /// Helper for generating ZK proof fixtures: prints the bs58 account for
    /// `moonlight_account(1)`. Run with:
    /// `cargo test print_fixture_account -- --ignored --nocapture`
    #[test]
    #[ignore]
    pub fn print_fixture_account() {
        let account = moonlight_account(1);
        println!("{}", account_string(&account));
        let hex: String = account
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        println!("{hex}");
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

    // --- ZK proven-run fixtures (generated by zk/prover, see fixtures/) ---
    // Account: moonlight_account(1); game seed 42; zero-input trace.
    const FIXTURE_GAME_SEED: u64 = 42;
    const FIXTURE_SCORE: u64 = 26;
    const FIXTURE_TICKS: u32 = 226;
    const FIXTURE_PROOF: &[u8] = include_bytes!("../fixtures/proof.bin");

    fn routed_submit_run(
        session: &mut Session,
        sender: PublicKey,
        dario_id: ContractId,
        seed: u64,
        score: u64,
        ticks: u32,
        proof: Vec<u8>,
    ) -> Result<(), Error> {
        with_public_sender(session, sender)?;
        session.call::<_, ()>(
            TRANSFER_CONTRACT,
            "submit_run",
            &(dario_id, seed, score, ticks, proof),
            LIMIT,
        )?;
        Ok(())
    }

    #[test]
    pub fn test_submit_run_verifies_real_proof() -> Result<(), Error> {
        let (mut session, dario_id) = setup_with_moonlight_router()?;
        let wallet = moonlight_account(1);
        let account = account_string(&wallet);

        // No proven runs yet.
        assert_eq!(
            session
                .call::<_, u64>(dario_id, "best_score_for", &account, LIMIT)?
                .data,
            0
        );

        routed_submit_run(
            &mut session,
            wallet,
            dario_id,
            FIXTURE_GAME_SEED,
            FIXTURE_SCORE,
            FIXTURE_TICKS,
            FIXTURE_PROOF.to_vec(),
        )?;

        assert_eq!(
            session
                .call::<_, u64>(dario_id, "best_score_for", &account, LIMIT)?
                .data,
            FIXTURE_SCORE
        );
        assert_eq!(
            session
                .call::<_, u32>(dario_id, "proven_runs_for", &account, LIMIT)?
                .data,
            1
        );

        let leaderboard = session
            .call::<_, Vec<(String, u64, u32)>>(dario_id, "leaderboard", &(), LIMIT)?
            .data;
        assert_eq!(leaderboard, vec![(account, FIXTURE_SCORE, 1)]);

        Ok(())
    }

    #[test]
    pub fn test_submit_run_rejects_replayed_seed() -> Result<(), Error> {
        let (mut session, dario_id) = setup_with_moonlight_router()?;
        let wallet = moonlight_account(1);

        routed_submit_run(
            &mut session,
            wallet,
            dario_id,
            FIXTURE_GAME_SEED,
            FIXTURE_SCORE,
            FIXTURE_TICKS,
            FIXTURE_PROOF.to_vec(),
        )?;

        // Same proof again: seed replay must be rejected.
        let result = routed_submit_run(
            &mut session,
            wallet,
            dario_id,
            FIXTURE_GAME_SEED,
            FIXTURE_SCORE,
            FIXTURE_TICKS,
            FIXTURE_PROOF.to_vec(),
        );
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    pub fn test_submit_run_rejects_tampered_claims() -> Result<(), Error> {
        let (mut session, dario_id) = setup_with_moonlight_router()?;
        let wallet = moonlight_account(1);

        // Changed score within the ranked range: journal no longer matches
        // the proof, so this exercises proof binding rather than range checks.
        let result = routed_submit_run(
            &mut session,
            wallet,
            dario_id,
            FIXTURE_GAME_SEED,
            FIXTURE_SCORE + 1,
            FIXTURE_TICKS,
            FIXTURE_PROOF.to_vec(),
        );
        assert!(result.is_err());

        // Wrong sender: proof binds moonlight_account(1), not (2).
        let result = routed_submit_run(
            &mut session,
            moonlight_account(2),
            dario_id,
            FIXTURE_GAME_SEED,
            FIXTURE_SCORE,
            FIXTURE_TICKS,
            FIXTURE_PROOF.to_vec(),
        );
        assert!(result.is_err());

        Ok(())
    }

    // --- Browser (circom/snarkjs) proven-run fixtures ---
    // Generated by dash_zk export_input + snarkjs + zk_browser/js/ark-proof.mjs.
    // Account: moonlight_account(1); game seed 42; autopilot trace.
    const ZK_FIXTURE_GAME_SEED: u64 = 42;
    const ZK_FIXTURE_SCORE: u64 = 743;
    const ZK_FIXTURE_TICKS: u32 = 997;
    const ZK_FIXTURE_PROOF: &[u8] = include_bytes!("../fixtures/zk_proof.bin");

    fn routed_submit_zk_run(
        session: &mut Session,
        sender: PublicKey,
        dario_id: ContractId,
        seed: u64,
        score: u64,
        ticks: u32,
        proof: Vec<u8>,
    ) -> Result<(), Error> {
        with_public_sender(session, sender)?;
        session.call::<_, ()>(
            TRANSFER_CONTRACT,
            "submit_zk_run",
            &(dario_id, seed, score, ticks, proof),
            LIMIT,
        )?;
        Ok(())
    }

    #[test]
    pub fn test_submit_zk_run_verifies_browser_proof() -> Result<(), Error> {
        let (mut session, dario_id) = setup_with_moonlight_router()?;
        let wallet = moonlight_account(1);
        let account = account_string(&wallet);

        routed_submit_zk_run(
            &mut session,
            wallet,
            dario_id,
            ZK_FIXTURE_GAME_SEED,
            ZK_FIXTURE_SCORE,
            ZK_FIXTURE_TICKS,
            ZK_FIXTURE_PROOF.to_vec(),
        )?;

        assert_eq!(
            session
                .call::<_, u64>(dario_id, "best_score_for", &account, LIMIT)?
                .data,
            ZK_FIXTURE_SCORE
        );
        assert_eq!(
            session
                .call::<_, u32>(dario_id, "proven_runs_for", &account, LIMIT)?
                .data,
            1
        );

        Ok(())
    }

    #[test]
    pub fn test_submit_zk_run_rejects_replayed_seed() -> Result<(), Error> {
        let (mut session, dario_id) = setup_with_moonlight_router()?;
        let wallet = moonlight_account(1);

        routed_submit_zk_run(
            &mut session,
            wallet,
            dario_id,
            ZK_FIXTURE_GAME_SEED,
            ZK_FIXTURE_SCORE,
            ZK_FIXTURE_TICKS,
            ZK_FIXTURE_PROOF.to_vec(),
        )?;

        let result = routed_submit_zk_run(
            &mut session,
            wallet,
            dario_id,
            ZK_FIXTURE_GAME_SEED,
            ZK_FIXTURE_SCORE,
            ZK_FIXTURE_TICKS,
            ZK_FIXTURE_PROOF.to_vec(),
        );
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    pub fn test_submit_zk_run_rejects_tampered_claims() -> Result<(), Error> {
        let (mut session, dario_id) = setup_with_moonlight_router()?;
        let wallet = moonlight_account(1);

        // Changed score within the ranked range: public inputs no longer
        // match the proof, so this exercises proof binding rather than range
        // checks.
        let result = routed_submit_zk_run(
            &mut session,
            wallet,
            dario_id,
            ZK_FIXTURE_GAME_SEED,
            ZK_FIXTURE_SCORE + 1,
            ZK_FIXTURE_TICKS,
            ZK_FIXTURE_PROOF.to_vec(),
        );
        assert!(result.is_err());

        // Wrong seed: contract recomputes a different obstacle schedule.
        let result = routed_submit_zk_run(
            &mut session,
            wallet,
            dario_id,
            7,
            ZK_FIXTURE_SCORE,
            ZK_FIXTURE_TICKS,
            ZK_FIXTURE_PROOF.to_vec(),
        );
        assert!(result.is_err());

        // Wrong sender: proof binds moonlight_account(1), not (2).
        let result = routed_submit_zk_run(
            &mut session,
            moonlight_account(2),
            dario_id,
            ZK_FIXTURE_GAME_SEED,
            ZK_FIXTURE_SCORE,
            ZK_FIXTURE_TICKS,
            ZK_FIXTURE_PROOF.to_vec(),
        );
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    pub fn test_proven_run_paths_reject_scores_above_ranked_cap() -> Result<(), Error> {
        let (mut session, dario_id) = setup_with_moonlight_router()?;
        let score = dash_zk::MAX_RANKED_SCORE + 1;

        assert!(routed_submit_run(
            &mut session,
            moonlight_account(1),
            dario_id,
            FIXTURE_GAME_SEED,
            score,
            FIXTURE_TICKS,
            FIXTURE_PROOF.to_vec(),
        )
        .is_err());

        assert!(routed_submit_zk_run(
            &mut session,
            moonlight_account(2),
            dario_id,
            ZK_FIXTURE_GAME_SEED,
            score,
            ZK_FIXTURE_TICKS,
            ZK_FIXTURE_PROOF.to_vec(),
        )
        .is_err());

        Ok(())
    }
}
