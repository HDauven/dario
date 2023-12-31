use core::panic;

use piecrust::{contract_bytecode, ContractData, ContractId, Error, Session, SessionData, VM};

const OWNER: [u8; 32] = [0u8; 32];
const LIMIT: u64 = 1_000_000;

// Basic setup function that deals with VM instantiation, session setup and contract deployment
fn setup() -> Result<(Session, ContractId), Error> {
    let vm = VM::ephemeral()?;
    let mut session = vm.session(SessionData::builder())?;

    // Deploy the MarioFSM contract
    let mario_id = session.deploy(
        contract_bytecode!("contract"),
        ContractData::builder(OWNER),
        LIMIT,
    )?;

    Ok((session, mario_id))
}

#[test]
pub fn test_mario_fsm() -> Result<(), Error> {
    let Ok((mut session, mario_id)) = setup() else {
        panic!("Setup failed")
    };

    // Test case: Regular -> Super (Mushroom event)
    let receipt = session.call::<_, ()>(mario_id, "handle_event", &(0), LIMIT)?;
    // Check that there indeed is one event emitted
    assert_eq!(receipt.events.len(), 1);
    // Check that the event emitted has the topic "state"
    assert_eq!(receipt.events[0].topic, "state");
    // Check that Mario's state is 1 (Super)
    assert_eq!(receipt.events[0].data, 1_u32.to_le_bytes());

    // Test case: Super -> Fire (FireFlower event)
    let receipt = session.call::<_, ()>(mario_id, "handle_event", &(1), LIMIT)?;
    assert_eq!(receipt.events.len(), 1);
    assert_eq!(receipt.events[0].topic, "state");
    assert_eq!(receipt.events[0].data, 2_u32.to_le_bytes());

    Ok(())
}

#[test]
pub fn test_invalid_event() -> Result<(), Error> {
    let Ok((mut session, mario_id)) = setup() else {
        panic!("Setup failed")
    };

    // Test case: Invalid event number leads to a panic
    // Use an invalid event number
    let result = session.call::<_, ()>(mario_id, "handle_event", &(5), LIMIT);
    // Check if the call resulted in an error
    assert!(result.is_err());

    Ok(())
}

#[test]
pub fn test_read_state() -> Result<(), Error> {
    let Ok((mut session, mario_id)) = setup() else {
        panic!("Setup failed")
    };

    // Initial state should be Regular
    let receipt = session.call::<_, u32>(mario_id, "read_state", &(), LIMIT)?;
    assert_eq!(receipt.data, 0_u32);

    // Test state after Mushroom event
    session.call::<_, ()>(mario_id, "handle_event", &(0), LIMIT)?;
    let receipt = session.call::<_, u32>(mario_id, "read_state", &(), LIMIT)?;
    assert_eq!(receipt.data, 1_u32); // Super state

    // Test state after FireFlower event
    session.call::<_, ()>(mario_id, "handle_event", &(1), LIMIT)?;
    let receipt = session.call::<_, u32>(mario_id, "read_state", &(), LIMIT)?;
    assert_eq!(receipt.data, 2_u32); // Fire state

    Ok(())
}
