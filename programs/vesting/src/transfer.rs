use lee_core::{
    account::{AccountId, AccountWithMetadata, Data},
    program::{AccountPostState, ChainedCall},
};
use vesting_core::{VestingEvent, VestingSchedule};

/// Assigns the vesting position to a new beneficiary holding. Only allowed
/// when the schedule was created transferable; signed by the current
/// beneficiary.
pub fn transfer_beneficiary(
    schedule: AccountWithMetadata,
    beneficiary_holding: AccountWithMetadata,
    new_beneficiary_holding_id: AccountId,
) -> (Vec<AccountPostState>, Vec<ChainedCall>) {
    let schedule_data = VestingSchedule::try_from(&schedule.account.data)
        .expect("TransferBeneficiary: schedule account must contain a valid VestingSchedule");
    assert!(
        schedule_data.transferable,
        "TransferBeneficiary: position is not transferable"
    );
    assert!(
        beneficiary_holding.is_authorized,
        "TransferBeneficiary: current beneficiary must sign"
    );
    assert_eq!(
        beneficiary_holding.account_id, schedule_data.beneficiary_holding_id,
        "TransferBeneficiary: signer is not the current beneficiary"
    );
    assert_ne!(
        new_beneficiary_holding_id, schedule_data.beneficiary_holding_id,
        "TransferBeneficiary: new beneficiary must differ"
    );
    let schedule_post_data = VestingSchedule {
        beneficiary_holding_id: new_beneficiary_holding_id,
        ..schedule_data
    }
    .with_event(VestingEvent::BeneficiaryTransferred {
        new_beneficiary_holding_id,
    });
    let mut schedule_post = schedule.account;
    schedule_post.data = Data::from(&schedule_post_data);
    (
        vec![
            AccountPostState::new(schedule_post),
            AccountPostState::new(beneficiary_holding.account),
        ],
        vec![],
    )
}
