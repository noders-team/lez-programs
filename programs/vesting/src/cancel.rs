use clock_core::{ClockAccountData, CLOCK_01_PROGRAM_ACCOUNT_ID};
use lee_core::{
    account::{AccountWithMetadata, Data},
    program::{AccountPostState, ChainedCall},
};
use vesting_core::{compute_escrow_pda_seed, VestingEvent, VestingSchedule};

fn require_cancel_authority(schedule: &VestingSchedule, authority: &AccountWithMetadata) {
    assert!(
        authority.is_authorized,
        "Cancel: cancellation authority must sign"
    );
    assert_eq!(
        authority.account_id, schedule.cancel_authority_id,
        "Cancel: signer is not the nominated cancellation authority"
    );
}

/// Cancels a cancelable schedule. Unvested tokens return to the creator
/// holding; tokens vested up to now remain claimable by the beneficiary.
///
/// The universal ecosystem invariant: vested = earned, unvested = returned.
pub fn cancel(
    schedule: AccountWithMetadata,
    escrow: AccountWithMetadata,
    creator_holding: AccountWithMetadata,
    clock: AccountWithMetadata,
    cancel_authority: AccountWithMetadata,
) -> (Vec<AccountPostState>, Vec<ChainedCall>) {
    let schedule_data = VestingSchedule::try_from(&schedule.account.data)
        .expect("Cancel: schedule account must contain a valid VestingSchedule");
    require_cancel_authority(&schedule_data, &cancel_authority);
    assert!(
        schedule_data.cancelable,
        "Cancel: schedule is non-cancelable"
    );
    assert!(
        schedule_data.cancelled_at.is_none(),
        "Cancel: schedule is already cancelled"
    );
    assert_eq!(
        escrow.account_id, schedule_data.escrow_id,
        "Cancel: escrow account does not match the schedule"
    );
    assert_eq!(
        creator_holding.account_id, schedule_data.creator_holding_id,
        "Cancel: creator holding does not match the schedule"
    );
    assert_eq!(
        clock.account_id, CLOCK_01_PROGRAM_ACCOUNT_ID,
        "Cancel: clock account must be the canonical 1-block LEZ clock account"
    );

    let now = ClockAccountData::from_bytes(clock.account.data.as_ref()).timestamp;
    let vested = schedule_data.vested_at(now);
    let returned = schedule_data
        .total_amount
        .checked_sub(vested)
        .expect("Cancel: vested cannot exceed total");

    let schedule_post_data = VestingSchedule {
        cancelled_at: Some(now),
        ..schedule_data
    }
    .with_event(VestingEvent::Cancelled { returned });
    let mut schedule_post = schedule.account;
    schedule_post.data = Data::from(&schedule_post_data);

    // Every declared account is echoed in the output (LEZ v0.2.4 rule).
    let escrow_echo = escrow.account.clone();
    let creator_echo = creator_holding.account.clone();
    let mut chained = Vec::new();
    if returned > 0 {
        let mut escrow_auth = escrow;
        escrow_auth.is_authorized = true;
        chained.push(
            ChainedCall::new(
                schedule_post_data.token_program_id,
                vec![escrow_auth, creator_holding],
                &token_core::Instruction::Transfer {
                    amount_to_transfer: returned,
                },
            )
            .with_pda_seeds(vec![compute_escrow_pda_seed(schedule.account_id)]),
        );
    }

    (
        vec![
            AccountPostState::new(schedule_post),
            AccountPostState::new(escrow_echo),
            AccountPostState::new(creator_echo),
            AccountPostState::new(clock.account),
            AccountPostState::new(cancel_authority.account),
        ],
        chained,
    )
}

/// Irreversibly converts a cancelable schedule into a non-cancelable one
/// (a credible-commitment signal to the beneficiary).
pub fn make_non_cancelable(
    schedule: AccountWithMetadata,
    cancel_authority: AccountWithMetadata,
) -> (Vec<AccountPostState>, Vec<ChainedCall>) {
    let schedule_data = VestingSchedule::try_from(&schedule.account.data)
        .expect("MakeNonCancelable: schedule account must contain a valid VestingSchedule");
    require_cancel_authority(&schedule_data, &cancel_authority);
    assert!(
        schedule_data.cancelable,
        "MakeNonCancelable: schedule is already non-cancelable"
    );
    let schedule_post_data = VestingSchedule {
        cancelable: false,
        ..schedule_data
    }
    .with_event(VestingEvent::MadeNonCancelable);
    let mut schedule_post = schedule.account;
    schedule_post.data = Data::from(&schedule_post_data);
    (
        vec![
            AccountPostState::new(schedule_post),
            AccountPostState::new(cancel_authority.account),
        ],
        vec![],
    )
}
