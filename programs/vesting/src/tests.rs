#![cfg(test)]

use clock_core::{ClockAccountData, CLOCK_01_PROGRAM_ACCOUNT_ID};
use lee_core::{
    account::{Account, AccountId, AccountWithMetadata, Data, Nonce},
    program::ProgramId,
    Timestamp,
};
use token_core::TokenHolding;
use vesting_core::{
    compute_escrow_pda, ScheduleKind, ScheduleParams, VestingEvent, VestingSchedule,
};

use crate::{
    cancel::{cancel, make_non_cancelable},
    claim::{claim, claim_to, claim_to_private},
    create_schedule::{build_schedule, create_schedule, create_schedule_batch},
    milestone::signal_milestone,
    transfer::transfer_beneficiary,
};

const VESTING_PROGRAM_ID: ProgramId = [7u32; 8];
const TOKEN_PROGRAM_ID: ProgramId = [5u32; 8];

const TOTAL: u128 = 1_000;
const START: Timestamp = 10_000;
const CLIFF: Timestamp = 20_000;
const END: Timestamp = 30_000;

fn definition_id() -> AccountId {
    AccountId::new([21; 32])
}

fn schedule_id() -> AccountId {
    AccountId::new([22; 32])
}

fn creator_holding_id() -> AccountId {
    AccountId::new([23; 32])
}

fn beneficiary_holding_id() -> AccountId {
    AccountId::new([24; 32])
}

fn fresh_account(account_id: AccountId, is_authorized: bool) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account::default(),
        is_authorized,
        account_id,
    }
}

fn holding_account(
    account_id: AccountId,
    balance: u128,
    is_authorized: bool,
) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account {
            program_owner: TOKEN_PROGRAM_ID,
            balance: 0u128,
            data: Data::from(&TokenHolding::Fungible {
                definition_id: definition_id(),
                balance,
            }),
            nonce: Nonce(0),
        },
        is_authorized,
        account_id,
    }
}

fn clock_account(timestamp: Timestamp) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account {
            program_owner: [0u32; 8],
            balance: 0u128,
            data: Data::try_from(
                ClockAccountData {
                    block_id: 1,
                    timestamp,
                }
                .to_bytes(),
            )
            .expect("clock data fits into Data"),
            nonce: Nonce(0),
        },
        is_authorized: false,
        account_id: CLOCK_01_PROGRAM_ACCOUNT_ID,
    }
}

fn escrow_id() -> AccountId {
    compute_escrow_pda(VESTING_PROGRAM_ID, schedule_id())
}

fn params() -> ScheduleParams {
    ScheduleParams {
        beneficiary_holding_id: beneficiary_holding_id(),
        total_amount: TOTAL,
        kind: ScheduleKind::CliffLinear {
            start: START,
            cliff: CLIFF,
            end: END,
        },
        cancelable: true,
        transferable: false,
        cancel_authority_id: None,
        milestone_authority_id: None,
        token_program_id: TOKEN_PROGRAM_ID,
    }
}

fn created_schedule() -> VestingSchedule {
    build_schedule(creator_holding_id(), escrow_id(), &params())
}

fn schedule_account(schedule: &VestingSchedule) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account {
            program_owner: VESTING_PROGRAM_ID,
            balance: 0u128,
            data: Data::from(schedule),
            nonce: Nonce(0),
        },
        is_authorized: false,
        account_id: schedule_id(),
    }
}

fn escrow_holding(balance: u128) -> AccountWithMetadata {
    holding_account(escrow_id(), balance, false)
}

fn run_create_schedule() -> (
    Vec<lee_core::program::AccountPostState>,
    Vec<lee_core::program::ChainedCall>,
) {
    create_schedule(
        fresh_account(schedule_id(), true),
        holding_account(creator_holding_id(), TOTAL, true),
        fresh_account(escrow_id(), false),
        params(),
        VESTING_PROGRAM_ID,
    )
}

#[test]
fn create_schedule_writes_state_and_funds_escrow() {
    let (posts, calls) = run_create_schedule();

    assert_eq!(
        posts.len(),
        3,
        "schedule, creator holding echo, escrow echo"
    );
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(schedule, created_schedule());

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].program_id, TOKEN_PROGRAM_ID);
    assert_eq!(calls[0].pre_states.len(), 2);
    assert_eq!(calls[0].pre_states[0].account_id, creator_holding_id());
    assert_eq!(calls[0].pre_states[1].account_id, escrow_id());
    assert!(
        calls[0].pre_states[1].is_authorized,
        "escrow must be PDA-authorized for the funding transfer"
    );
    assert_eq!(calls[0].pda_seeds.len(), 1);
}

#[test]
#[should_panic(expected = "escrow Account ID does not match PDA")]
fn create_schedule_rejects_wrong_escrow() {
    create_schedule(
        fresh_account(schedule_id(), true),
        holding_account(creator_holding_id(), TOTAL, true),
        fresh_account(AccountId::new([99; 32]), false),
        params(),
        VESTING_PROGRAM_ID,
    );
}

#[test]
#[should_panic(expected = "require start <= cliff <= end")]
fn create_schedule_rejects_inverted_times() {
    create_schedule(
        fresh_account(schedule_id(), true),
        holding_account(creator_holding_id(), TOTAL, true),
        fresh_account(escrow_id(), false),
        ScheduleParams {
            kind: ScheduleKind::CliffLinear {
                start: CLIFF,
                cliff: START,
                end: END,
            },
            ..params()
        },
        VESTING_PROGRAM_ID,
    );
}

#[test]
#[should_panic(expected = "nothing is claimable yet")]
fn claim_before_cliff_is_rejected() {
    claim(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, true),
        clock_account(CLIFF - 1),
    );
}

#[test]
fn claim_mid_schedule_pays_pro_rata_and_records_it() {
    // Midpoint of [START, END] => half of TOTAL is vested.
    let (posts, calls) = claim(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, true),
        clock_account((START + END) / 2),
    );

    // Schedule post-state first, then the declared accounts echoed unchanged
    // (escrow, beneficiary holding, clock) per the v0.2.4 output rule.
    assert_eq!(posts.len(), 4);
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(schedule.claimed_amount, TOTAL / 2);
    assert_eq!(posts[1].account().data, escrow_holding(TOTAL).account.data);
    assert_eq!(
        posts[3].account().data,
        clock_account((START + END) / 2).account.data
    );

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].program_id, TOKEN_PROGRAM_ID);
    assert_eq!(calls[0].pre_states[0].account_id, escrow_id());
    assert!(
        calls[0].pre_states[0].is_authorized,
        "escrow must be PDA-authorized for the payout transfer"
    );
    assert_eq!(calls[0].pre_states[1].account_id, beneficiary_holding_id());
    assert_eq!(calls[0].pda_seeds.len(), 1);
}

#[test]
fn second_claim_pays_only_the_remainder() {
    let mut schedule = created_schedule();
    schedule.claimed_amount = TOTAL / 2;

    let (posts, _calls) = claim(
        schedule_account(&schedule),
        escrow_holding(TOTAL / 2),
        holding_account(beneficiary_holding_id(), TOTAL / 2, true),
        clock_account(END),
    );

    let updated =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(updated.claimed_amount, TOTAL);
}

#[test]
#[should_panic(expected = "nothing is claimable yet")]
fn fully_claimed_schedule_rejects_further_claims() {
    let mut schedule = created_schedule();
    schedule.claimed_amount = TOTAL;

    claim(
        schedule_account(&schedule),
        escrow_holding(0),
        holding_account(beneficiary_holding_id(), TOTAL, true),
        clock_account(END + 1),
    );
}

#[test]
fn create_schedule_records_created_event_and_default_authorities() {
    let (posts, _) = run_create_schedule();
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(schedule.event_seq, 0);
    assert_eq!(
        schedule.last_event,
        VestingEvent::Created {
            total_amount: TOTAL
        }
    );
    assert_eq!(schedule.cancel_authority_id, creator_holding_id());
    assert_eq!(schedule.milestone_authority_id, creator_holding_id());
}

#[test]
fn create_schedule_honours_nominated_authorities() {
    let cancel_auth = AccountId::new([31; 32]);
    let milestone_auth = AccountId::new([32; 32]);
    let (posts, _) = create_schedule(
        fresh_account(schedule_id(), true),
        holding_account(creator_holding_id(), TOTAL, true),
        fresh_account(escrow_id(), false),
        ScheduleParams {
            cancel_authority_id: Some(cancel_auth),
            milestone_authority_id: Some(milestone_auth),
            ..params()
        },
        VESTING_PROGRAM_ID,
    );
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(schedule.cancel_authority_id, cancel_auth);
    assert_eq!(schedule.milestone_authority_id, milestone_auth);
}

#[test]
fn claim_records_claimed_event() {
    let (posts, _) = claim(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, true),
        clock_account((START + END) / 2),
    );
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(schedule.event_seq, 1);
    assert_eq!(
        schedule.last_event,
        VestingEvent::Claimed { amount: TOTAL / 2 }
    );
}

fn private_destination_id() -> AccountId {
    AccountId::new([9u8; 32])
}

#[test]
fn claim_to_private_pays_vested_at_as_of_and_pins_window() {
    let as_of = (START + END) / 2;
    let (posts, calls, window) = claim_to_private(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, true),
        fresh_account(private_destination_id(), true),
        as_of,
    );

    assert_eq!(posts.len(), 4, "schedule, escrow, beneficiary, destination");
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(schedule.claimed_amount, TOTAL / 2);
    assert_eq!(
        schedule.last_event,
        VestingEvent::Claimed { amount: TOTAL / 2 }
    );
    assert_eq!(
        posts[3].account(),
        &Account::default(),
        "destination echoed"
    );

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].program_id, TOKEN_PROGRAM_ID);
    assert!(
        calls[0].pre_states[0].is_authorized,
        "escrow PDA-authorized"
    );
    assert_eq!(calls[0].pre_states[1].account_id, private_destination_id());

    assert!(window.is_valid_for(as_of));
    assert!(!window.is_valid_for(as_of - 1));
    assert!(window.is_valid_for(u64::MAX), "no upper bound");
}

#[test]
#[should_panic(expected = "nothing is claimable")]
fn claim_to_private_rejects_as_of_before_cliff() {
    claim_to_private(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, true),
        fresh_account(private_destination_id(), true),
        CLIFF - 1,
    );
}

#[test]
#[should_panic(expected = "beneficiary must sign")]
fn claim_to_private_requires_beneficiary_signature() {
    claim_to_private(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, false),
        fresh_account(private_destination_id(), true),
        END,
    );
}

#[test]
fn claim_to_private_respects_cancellation_freeze() {
    let mut schedule = created_schedule();
    schedule.cancelled_at = Some((START + END) / 2);
    let (posts, _calls, _window) = claim_to_private(
        schedule_account(&schedule),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, true),
        fresh_account(private_destination_id(), true),
        END + 1_000,
    );
    let updated =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(
        updated.claimed_amount,
        TOTAL / 2,
        "accrual frozen at cancelled_at"
    );
}

fn authority_account(id: AccountId, is_authorized: bool) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account::default(),
        is_authorized,
        account_id: id,
    }
}

// ── Cancel / MakeNonCancelable ──────────────────────────────────────────

#[test]
fn cancel_returns_unvested_and_keeps_vested_claimable() {
    // Midpoint: 500 vested, 0 claimed → 500 returned to creator.
    let (posts, calls) = cancel(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(creator_holding_id(), 0, false),
        clock_account((START + END) / 2),
        authority_account(creator_holding_id(), true),
    );
    assert_eq!(posts.len(), 5);
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(schedule.cancelled_at, Some((START + END) / 2));
    assert_eq!(
        schedule.last_event,
        VestingEvent::Cancelled {
            returned: TOTAL / 2
        }
    );
    assert_eq!(
        schedule.claimable_at(END),
        TOTAL / 2,
        "vested stays claimable"
    );
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].pre_states[0].account_id, escrow_id());
    assert!(calls[0].pre_states[0].is_authorized);
    assert_eq!(calls[0].pre_states[1].account_id, creator_holding_id());
}

#[test]
fn cancel_after_end_returns_nothing_and_skips_transfer() {
    let (posts, calls) = cancel(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(creator_holding_id(), 0, false),
        clock_account(END + 1),
        authority_account(creator_holding_id(), true),
    );
    assert!(calls.is_empty());
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(schedule.last_event, VestingEvent::Cancelled { returned: 0 });
}

#[test]
#[should_panic(expected = "cancellation authority must sign")]
fn cancel_requires_authority_signature() {
    cancel(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(creator_holding_id(), 0, false),
        clock_account(START),
        authority_account(creator_holding_id(), false),
    );
}

#[test]
#[should_panic(expected = "not the nominated cancellation authority")]
fn cancel_rejects_wrong_authority() {
    cancel(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(creator_holding_id(), 0, false),
        clock_account(START),
        authority_account(AccountId::new([99; 32]), true),
    );
}

#[test]
#[should_panic(expected = "schedule is non-cancelable")]
fn cancel_rejects_non_cancelable() {
    let s = VestingSchedule {
        cancelable: false,
        ..created_schedule()
    };
    cancel(
        schedule_account(&s),
        escrow_holding(TOTAL),
        holding_account(creator_holding_id(), 0, false),
        clock_account(START),
        authority_account(creator_holding_id(), true),
    );
}

#[test]
#[should_panic(expected = "schedule is already cancelled")]
fn cancel_is_rejected_twice() {
    let s = VestingSchedule {
        cancelled_at: Some(START),
        ..created_schedule()
    };
    cancel(
        schedule_account(&s),
        escrow_holding(TOTAL),
        holding_account(creator_holding_id(), 0, false),
        clock_account(CLIFF),
        authority_account(creator_holding_id(), true),
    );
}

#[test]
fn make_non_cancelable_is_one_way() {
    let (posts, calls) = make_non_cancelable(
        schedule_account(&created_schedule()),
        authority_account(creator_holding_id(), true),
    );
    assert!(calls.is_empty());
    assert_eq!(posts.len(), 2);
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert!(!schedule.cancelable);
    assert_eq!(schedule.last_event, VestingEvent::MadeNonCancelable);
}

#[test]
#[should_panic(expected = "already non-cancelable")]
fn make_non_cancelable_rejects_repeat() {
    let s = VestingSchedule {
        cancelable: false,
        ..created_schedule()
    };
    make_non_cancelable(
        schedule_account(&s),
        authority_account(creator_holding_id(), true),
    );
}

// ── SignalMilestone ────────────────────────────────────────────────────

fn milestone_schedule() -> VestingSchedule {
    build_schedule(
        creator_holding_id(),
        escrow_id(),
        &ScheduleParams {
            kind: ScheduleKind::Milestone {
                tranches: vec![100, 300, 600],
            },
            ..params()
        },
    )
}

#[test]
fn signal_milestone_unlocks_next_tranche() {
    let (posts, calls) = signal_milestone(
        schedule_account(&milestone_schedule()),
        authority_account(creator_holding_id(), true),
        0,
    );
    assert!(calls.is_empty());
    assert_eq!(posts.len(), 2);
    let s = VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(s.milestones_signalled, 1);
    assert_eq!(s.claimable_at(0), 100);
    assert_eq!(
        s.last_event,
        VestingEvent::MilestoneSignalled {
            index: 0,
            amount: 100
        }
    );
}

#[test]
#[should_panic(expected = "milestone already signalled")]
fn signal_milestone_is_idempotent_per_index() {
    let s = VestingSchedule {
        milestones_signalled: 1,
        ..milestone_schedule()
    };
    signal_milestone(
        schedule_account(&s),
        authority_account(creator_holding_id(), true),
        0,
    );
}

#[test]
#[should_panic(expected = "milestones must be signalled in order")]
fn signal_milestone_rejects_skipping() {
    signal_milestone(
        schedule_account(&milestone_schedule()),
        authority_account(creator_holding_id(), true),
        2,
    );
}

#[test]
#[should_panic(expected = "milestone authority must sign")]
fn signal_milestone_requires_signature() {
    signal_milestone(
        schedule_account(&milestone_schedule()),
        authority_account(creator_holding_id(), false),
        0,
    );
}

#[test]
#[should_panic(expected = "not a milestone schedule")]
fn signal_milestone_rejects_time_based_schedule() {
    signal_milestone(
        schedule_account(&created_schedule()),
        authority_account(creator_holding_id(), true),
        0,
    );
}

#[test]
#[should_panic(expected = "schedule is cancelled")]
fn signal_milestone_rejects_cancelled_schedule() {
    let s = VestingSchedule {
        cancelled_at: Some(START),
        ..milestone_schedule()
    };
    signal_milestone(
        schedule_account(&s),
        authority_account(creator_holding_id(), true),
        0,
    );
}

#[test]
fn claim_pays_signalled_milestone_tranches() {
    let s = VestingSchedule {
        milestones_signalled: 2,
        ..milestone_schedule()
    };
    let (posts, calls) = claim(
        schedule_account(&s),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, true),
        clock_account(START),
    );
    let s = VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(s.claimed_amount, 400);
    assert_eq!(calls.len(), 1);
}

// ── TransferBeneficiary ────────────────────────────────────────────────

#[test]
fn transfer_beneficiary_moves_position() {
    let s = VestingSchedule {
        transferable: true,
        ..created_schedule()
    };
    let new_id = AccountId::new([77; 32]);
    let (posts, calls) = transfer_beneficiary(
        schedule_account(&s),
        holding_account(beneficiary_holding_id(), 0, true),
        new_id,
    );
    assert!(calls.is_empty());
    assert_eq!(posts.len(), 2);
    let s = VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(s.beneficiary_holding_id, new_id);
    assert_eq!(
        s.last_event,
        VestingEvent::BeneficiaryTransferred {
            new_beneficiary_holding_id: new_id
        }
    );
}

#[test]
#[should_panic(expected = "position is not transferable")]
fn transfer_beneficiary_rejects_non_transferable() {
    transfer_beneficiary(
        schedule_account(&created_schedule()),
        holding_account(beneficiary_holding_id(), 0, true),
        AccountId::new([77; 32]),
    );
}

#[test]
#[should_panic(expected = "current beneficiary must sign")]
fn transfer_beneficiary_requires_signature() {
    let s = VestingSchedule {
        transferable: true,
        ..created_schedule()
    };
    transfer_beneficiary(
        schedule_account(&s),
        holding_account(beneficiary_holding_id(), 0, false),
        AccountId::new([77; 32]),
    );
}

#[test]
#[should_panic(expected = "signer is not the current beneficiary")]
fn transfer_beneficiary_rejects_wrong_signer() {
    let s = VestingSchedule {
        transferable: true,
        ..created_schedule()
    };
    transfer_beneficiary(
        schedule_account(&s),
        holding_account(creator_holding_id(), 0, true),
        AccountId::new([77; 32]),
    );
}

// ── CreateScheduleBatch ────────────────────────────────────────────────

fn batch_ids(n: u8) -> Vec<(AccountId, AccountId)> {
    (0..n)
        .map(|i| {
            let schedule = AccountId::new([100u8.saturating_add(i); 32]);
            (schedule, compute_escrow_pda(VESTING_PROGRAM_ID, schedule))
        })
        .collect()
}

#[test]
fn batch_creates_one_schedule_per_pair_and_one_transfer_each() {
    let ids = batch_ids(3);
    let rest: Vec<AccountWithMetadata> = ids
        .iter()
        .flat_map(|(s, e)| [fresh_account(*s, true), fresh_account(*e, false)])
        .collect();
    let (posts, calls) = create_schedule_batch(
        holding_account(creator_holding_id(), 3_000, true),
        rest,
        vec![params(), params(), params()],
        VESTING_PROGRAM_ID,
    );
    assert_eq!(posts.len(), 7);
    assert_eq!(calls.len(), 3);
    for (i, (_, escrow)) in ids.iter().enumerate() {
        let s = VestingSchedule::try_from(&posts[1 + 2 * i].account().data)
            .expect("schedule data must parse");
        assert_eq!(s.escrow_id, *escrow);
        assert_eq!(calls[i].pre_states[1].account_id, *escrow);
    }
}

#[test]
#[should_panic(expected = "expected one schedule/escrow pair per params entry")]
fn batch_rejects_mismatched_account_count() {
    let ids = batch_ids(1);
    let rest = vec![fresh_account(ids[0].0, true)];
    create_schedule_batch(
        holding_account(creator_holding_id(), TOTAL, true),
        rest,
        vec![params()],
        VESTING_PROGRAM_ID,
    );
}

#[test]
#[should_panic(expected = "batch must not be empty")]
fn batch_rejects_empty() {
    create_schedule_batch(
        holding_account(creator_holding_id(), TOTAL, true),
        vec![],
        vec![],
        VESTING_PROGRAM_ID,
    );
}

// ── Claim authorisation / ClaimTo ──────────────────────────────────────

#[test]
#[should_panic(expected = "beneficiary must sign")]
fn claim_requires_beneficiary_signature() {
    claim(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, false),
        clock_account((START + END) / 2),
    );
}

#[test]
fn claim_to_pays_the_destination_and_echoes_all_accounts() {
    let destination = AccountId::new([88; 32]);
    let (posts, calls) = claim_to(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, true),
        fresh_account(destination, true),
        clock_account((START + END) / 2),
    );
    assert_eq!(posts.len(), 5);
    let schedule =
        VestingSchedule::try_from(&posts[0].account().data).expect("schedule data must parse");
    assert_eq!(schedule.claimed_amount, TOTAL / 2);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].pre_states[0].account_id, escrow_id());
    assert_eq!(calls[0].pre_states[1].account_id, destination);
}

#[test]
#[should_panic(expected = "beneficiary must sign")]
fn claim_to_requires_beneficiary_signature() {
    claim_to(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, false),
        fresh_account(AccountId::new([88; 32]), true),
        clock_account(END),
    );
}

#[test]
#[should_panic(expected = "use Claim to pay the registered beneficiary holding")]
fn claim_to_rejects_the_registered_holding_as_destination() {
    claim_to(
        schedule_account(&created_schedule()),
        escrow_holding(TOTAL),
        holding_account(beneficiary_holding_id(), 0, true),
        holding_account(beneficiary_holding_id(), 0, false),
        clock_account(END),
    );
}

#[test]
#[should_panic(expected = "at most MAX_BATCH_SCHEDULES schedules per transaction")]
fn batch_rejects_more_than_the_documented_maximum() {
    let ids = batch_ids(11);
    let rest: Vec<AccountWithMetadata> = ids
        .iter()
        .flat_map(|(s, e)| [fresh_account(*s, true), fresh_account(*e, false)])
        .collect();
    create_schedule_batch(
        holding_account(creator_holding_id(), 11_000, true),
        rest,
        (0..11).map(|_| params()).collect(),
        VESTING_PROGRAM_ID,
    );
}
