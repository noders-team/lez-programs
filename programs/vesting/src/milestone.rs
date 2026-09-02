use lee_core::{
    account::{AccountWithMetadata, Data},
    program::{AccountPostState, ChainedCall},
};
use vesting_core::{ScheduleKind, VestingEvent, VestingSchedule};

/// Unlocks milestone `index`; the beneficiary then claims it with `Claim`.
///
/// Milestones are signalled strictly in order and each index at most once,
/// so a repeated signal fails deterministically and never double-unlocks.
pub fn signal_milestone(
    schedule: AccountWithMetadata,
    milestone_authority: AccountWithMetadata,
    index: u32,
) -> (Vec<AccountPostState>, Vec<ChainedCall>) {
    let schedule_data = VestingSchedule::try_from(&schedule.account.data)
        .expect("SignalMilestone: schedule account must contain a valid VestingSchedule");
    assert!(
        milestone_authority.is_authorized,
        "SignalMilestone: milestone authority must sign"
    );
    assert_eq!(
        milestone_authority.account_id, schedule_data.milestone_authority_id,
        "SignalMilestone: signer is not the nominated milestone authority"
    );
    assert!(
        schedule_data.cancelled_at.is_none(),
        "SignalMilestone: schedule is cancelled"
    );
    let tranches = match &schedule_data.kind {
        ScheduleKind::Milestone { tranches } => tranches,
        ScheduleKind::CliffLinear { .. } => {
            panic!("SignalMilestone: not a milestone schedule")
        }
    };
    assert!(
        index >= schedule_data.milestones_signalled,
        "SignalMilestone: milestone already signalled"
    );
    assert!(
        index == schedule_data.milestones_signalled,
        "SignalMilestone: milestones must be signalled in order"
    );
    let position = usize::try_from(index).expect("milestone index fits usize");
    let amount = *tranches
        .get(position)
        .expect("SignalMilestone: milestone index out of range");

    let schedule_post_data = VestingSchedule {
        milestones_signalled: schedule_data
            .milestones_signalled
            .checked_add(1)
            .expect("milestone counter cannot overflow"),
        ..schedule_data
    }
    .with_event(VestingEvent::MilestoneSignalled { index, amount });
    let mut schedule_post = schedule.account;
    schedule_post.data = Data::from(&schedule_post_data);
    (
        vec![
            AccountPostState::new(schedule_post),
            AccountPostState::new(milestone_authority.account),
        ],
        vec![],
    )
}
