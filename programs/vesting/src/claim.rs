use clock_core::{ClockAccountData, CLOCK_01_PROGRAM_ACCOUNT_ID};
use lee_core::{
    account::{AccountWithMetadata, Data},
    program::{AccountPostState, ChainedCall, TimestampValidityWindow},
};
use vesting_core::{compute_escrow_pda_seed, VestingEvent, VestingSchedule};

/// Claims all currently vested, unclaimed tokens to the beneficiary holding.
///
/// The beneficiary holding must sign: only the beneficiary decides when and
/// where vested tokens surface (a third party cannot force a public claim and
/// defeat a later private one), and the signature nonce makes every claim
/// transaction unique. Time is read from the canonical LEZ clock account.
pub fn claim(
    schedule: AccountWithMetadata,
    escrow: AccountWithMetadata,
    beneficiary_holding: AccountWithMetadata,
    clock: AccountWithMetadata,
) -> (Vec<AccountPostState>, Vec<ChainedCall>) {
    let (schedule_post, amount, token_program_id) =
        prepare_claim(&schedule, &escrow, &beneficiary_holding, &clock);
    let escrow_echo = escrow.account.clone();
    let beneficiary_echo = beneficiary_holding.account.clone();
    let pay_out = pay_out_call(
        escrow,
        beneficiary_holding,
        schedule.account_id,
        amount,
        token_program_id,
    );
    (
        vec![
            AccountPostState::new(schedule_post),
            AccountPostState::new(escrow_echo),
            AccountPostState::new(beneficiary_echo),
            AccountPostState::new(clock.account),
        ],
        vec![pay_out],
    )
}

/// Claims all currently vested, unclaimed tokens to `destination` — a public
/// holding or a fresh private account — authorised by the beneficiary holding.
pub fn claim_to(
    schedule: AccountWithMetadata,
    escrow: AccountWithMetadata,
    beneficiary_holding: AccountWithMetadata,
    destination: AccountWithMetadata,
    clock: AccountWithMetadata,
) -> (Vec<AccountPostState>, Vec<ChainedCall>) {
    let (schedule_post, amount, token_program_id) =
        prepare_claim(&schedule, &escrow, &beneficiary_holding, &clock);
    assert_ne!(
        destination.account_id, beneficiary_holding.account_id,
        "ClaimTo: use Claim to pay the registered beneficiary holding"
    );
    let escrow_echo = escrow.account.clone();
    let destination_echo = destination.account.clone();
    let pay_out = pay_out_call(
        escrow,
        destination,
        schedule.account_id,
        amount,
        token_program_id,
    );
    (
        vec![
            AccountPostState::new(schedule_post),
            AccountPostState::new(escrow_echo),
            AccountPostState::new(beneficiary_holding.account),
            AccountPostState::new(destination_echo),
            AccountPostState::new(clock.account),
        ],
        vec![pay_out],
    )
}

/// Validates the claim and returns the updated schedule account, the amount
/// to pay and the token program to pay it through.
fn prepare_claim(
    schedule: &AccountWithMetadata,
    escrow: &AccountWithMetadata,
    beneficiary_holding: &AccountWithMetadata,
    clock: &AccountWithMetadata,
) -> (
    lee_core::account::Account,
    u128,
    lee_core::program::ProgramId,
) {
    assert_eq!(
        clock.account_id, CLOCK_01_PROGRAM_ACCOUNT_ID,
        "Claim: clock account must be the canonical 1-block LEZ clock account"
    );
    let now = ClockAccountData::from_bytes(clock.account.data.as_ref()).timestamp;
    prepare_claim_at(schedule, escrow, beneficiary_holding, now)
}

/// Private claim: pays all tokens vested at `as_of` to `destination` (a fresh
/// private account) without reading the clock, and returns the timestamp
/// validity window `as_of..` the guest must pin on its output. The runtime
/// refuses to include the transaction before `as_of`, so the amount paid can
/// never exceed what is vested at inclusion time; because no clock account is
/// declared, the privacy-preserving proof is not bound to a per-block
/// pre-state and stays valid until included.
pub fn claim_to_private(
    schedule: AccountWithMetadata,
    escrow: AccountWithMetadata,
    beneficiary_holding: AccountWithMetadata,
    destination: AccountWithMetadata,
    as_of: u64,
) -> (
    Vec<AccountPostState>,
    Vec<ChainedCall>,
    TimestampValidityWindow,
) {
    let (schedule_post, amount, token_program_id) =
        prepare_claim_at(&schedule, &escrow, &beneficiary_holding, as_of);
    assert_ne!(
        destination.account_id, beneficiary_holding.account_id,
        "ClaimToPrivate: destination must differ from the registered holding"
    );
    let escrow_echo = escrow.account.clone();
    let destination_echo = destination.account.clone();
    let pay_out = pay_out_call(
        escrow,
        destination,
        schedule.account_id,
        amount,
        token_program_id,
    );
    (
        vec![
            AccountPostState::new(schedule_post),
            AccountPostState::new(escrow_echo),
            AccountPostState::new(beneficiary_holding.account),
            AccountPostState::new(destination_echo),
        ],
        vec![pay_out],
        TimestampValidityWindow::from(as_of..),
    )
}

/// Validates the claim at an explicit time `now` and returns the updated
/// schedule account, the amount to pay and the token program to pay through.
fn prepare_claim_at(
    schedule: &AccountWithMetadata,
    escrow: &AccountWithMetadata,
    beneficiary_holding: &AccountWithMetadata,
    now: u64,
) -> (
    lee_core::account::Account,
    u128,
    lee_core::program::ProgramId,
) {
    let schedule_data = VestingSchedule::try_from(&schedule.account.data)
        .expect("Claim: schedule account must contain a valid VestingSchedule");
    assert_eq!(
        escrow.account_id, schedule_data.escrow_id,
        "Claim: escrow account does not match the schedule"
    );
    assert_eq!(
        beneficiary_holding.account_id, schedule_data.beneficiary_holding_id,
        "Claim: beneficiary holding does not match the schedule"
    );
    assert!(
        beneficiary_holding.is_authorized,
        "Claim: beneficiary must sign"
    );

    let amount_to_claim = schedule_data.claimable_at(now);
    assert!(amount_to_claim > 0, "Claim: nothing is claimable yet");

    let token_program_id = schedule_data.token_program_id;
    let schedule_post_data = VestingSchedule {
        claimed_amount: schedule_data
            .claimed_amount
            .checked_add(amount_to_claim)
            .expect("Claim: claimed amount cannot exceed total"),
        ..schedule_data
    }
    .with_event(VestingEvent::Claimed {
        amount: amount_to_claim,
    });
    let mut schedule_post = schedule.account.clone();
    schedule_post.data = Data::from(&schedule_post_data);
    (schedule_post, amount_to_claim, token_program_id)
}

/// Chained Token Program transfer from the program-owned escrow vault
/// (authorised via its PDA seed) to `recipient`.
fn pay_out_call(
    escrow: AccountWithMetadata,
    recipient: AccountWithMetadata,
    schedule_id: lee_core::account::AccountId,
    amount: u128,
    token_program_id: lee_core::program::ProgramId,
) -> ChainedCall {
    let mut escrow_auth = escrow;
    escrow_auth.is_authorized = true;
    ChainedCall::new(
        token_program_id,
        vec![escrow_auth, recipient],
        &token_core::Instruction::Transfer {
            amount_to_transfer: amount,
        },
    )
    .with_pda_seeds(vec![compute_escrow_pda_seed(schedule_id)])
}
