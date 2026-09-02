//! Cycle-cost benchmark for the Vesting Program (RFP-017 research spike).
//!
//! Runs the real `vesting` guest ELF through the RISC Zero executor (no proving) and reports the
//! zkVM cycle split (user / paging / reserved / total) for every instruction — `CreateSchedule`,
//! `Claim`, `ClaimTo`, `ClaimToPrivate`, `Cancel`, `MakeNonCancelable`, `SignalMilestone`,
//! `TransferBeneficiary` and `CreateScheduleBatch` with n = 2 and n = 10 — plus whether each
//! completes under the on-chain `MAX_NUM_CYCLES_PUBLIC_EXECUTION = 32 MiCycles` budget. The
//! chained Token Program transfers execute in the token guest and are not included in these
//! numbers.
//!
//! Ignored by default. Run with:
//!
//! ```sh
//! cargo test --manifest-path programs/benchmark/Cargo.toml --test vesting_cycle_bench -- --ignored --nocapture
//! ```

use clock_core::{ClockAccountData, CLOCK_01_PROGRAM_ACCOUNT_ID};
use lee_core::{
    account::{Account, AccountId, AccountWithMetadata, Data},
    program::ProgramId,
};
use risc0_zkvm::{serde::to_vec, ExecutorEnv, ExecutorImpl};
use token_core::TokenHolding;
use vesting_core::{
    compute_escrow_pda, Instruction, ScheduleKind, ScheduleParams, VestingEvent, VestingSchedule,
};

/// The on-chain public-execution cycle ceiling (`MAX_NUM_CYCLES_PUBLIC_EXECUTION` in `lee`).
const PUBLIC_EXECUTION_CYCLE_LIMIT: u64 = 1024 * 1024 * 32;

const TOTAL: u128 = 1_000_000;
const START: u64 = 10_000;
const CLIFF: u64 = 20_000;
const END: u64 = 30_000;

struct Cycles {
    user: u64,
    paging: u64,
    reserved: u64,
    total: u64,
}

fn program_id() -> ProgramId {
    vesting_methods::VESTING_ID
}

fn token_program_id() -> ProgramId {
    [5u32; 8]
}

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

fn escrow_id() -> AccountId {
    compute_escrow_pda(program_id(), schedule_id())
}

fn fresh_account(account_id: AccountId, is_authorized: bool) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account::default(),
        is_authorized,
        account_id,
    }
}

fn holding_account(account_id: AccountId, balance: u128) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account {
            program_owner: token_program_id(),
            data: Data::from(&TokenHolding::Fungible {
                definition_id: definition_id(),
                balance,
            }),
            ..Account::default()
        },
        is_authorized: false,
        account_id,
    }
}

fn clock_account(timestamp: u64) -> AccountWithMetadata {
    let data = ClockAccountData {
        block_id: 0,
        timestamp,
    }
    .to_bytes();
    AccountWithMetadata {
        account: Account {
            data: Data::try_from(data).expect("clock data fits"),
            ..Account::default()
        },
        is_authorized: false,
        account_id: CLOCK_01_PROGRAM_ACCOUNT_ID,
    }
}

fn build_env<'a>(pre_states: &[AccountWithMetadata], instruction: &Instruction) -> ExecutorEnv<'a> {
    let instruction_data: Vec<u32> = to_vec(instruction).expect("instruction serializes");

    let mut builder = ExecutorEnv::builder();
    builder.write(&program_id()).expect("write program id");
    builder
        .write(&None::<ProgramId>)
        .expect("write caller program id");
    builder
        .write(&pre_states.to_vec())
        .expect("write pre-states");
    builder
        .write(&instruction_data)
        .expect("write instruction data");
    builder.session_limit(None);
    builder.build().expect("env builds")
}

fn run(pre_states: &[AccountWithMetadata], instruction: &Instruction) -> Cycles {
    let env = build_env(pre_states, instruction);
    let session = ExecutorImpl::from_elf(env, vesting_methods::VESTING_ELF)
        .expect("loads ELF")
        .run()
        .expect("guest executes without panicking");

    Cycles {
        user: session.user_cycles,
        paging: session.paging_cycles,
        reserved: session.reserved_cycles,
        total: session.total_cycles,
    }
}

fn report(label: &str, cycles: &Cycles) {
    let fits = cycles.total <= PUBLIC_EXECUTION_CYCLE_LIMIT;
    println!(
        "{label:<20} user={:>10} paging={:>9} reserved={:>9} total={:>10}  {}",
        cycles.user,
        cycles.paging,
        cycles.reserved,
        cycles.total,
        if fits {
            "fits 32Mi budget"
        } else {
            "EXCEEDS 32Mi budget"
        },
    );
}

fn schedule_account_with(schedule: &VestingSchedule) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account {
            program_owner: program_id(),
            data: Data::from(schedule),
            ..Account::default()
        },
        is_authorized: false,
        account_id: schedule_id(),
    }
}

fn cliff_linear_schedule() -> VestingSchedule {
    VestingSchedule {
        creator_holding_id: creator_holding_id(),
        beneficiary_holding_id: beneficiary_holding_id(),
        escrow_id: escrow_id(),
        token_program_id: token_program_id(),
        total_amount: TOTAL,
        claimed_amount: 0,
        kind: ScheduleKind::CliffLinear {
            start: START,
            cliff: CLIFF,
            end: END,
        },
        milestones_signalled: 0,
        cancelable: true,
        transferable: true,
        cancelled_at: None,
        cancel_authority_id: creator_holding_id(),
        milestone_authority_id: creator_holding_id(),
        event_seq: 0,
        last_event: VestingEvent::Created {
            total_amount: TOTAL,
        },
    }
}

fn milestone_schedule() -> VestingSchedule {
    VestingSchedule {
        kind: ScheduleKind::Milestone {
            tranches: vec![TOTAL / 4, TOTAL / 4, TOTAL / 2],
        },
        ..cliff_linear_schedule()
    }
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
        token_program_id: token_program_id(),
    }
}

fn authorized(mut account: AccountWithMetadata) -> AccountWithMetadata {
    account.is_authorized = true;
    account
}

fn batch_pre_states(n: u8) -> Vec<AccountWithMetadata> {
    let funded = TOTAL.checked_mul(u128::from(n)).expect("fits");
    let mut pre = vec![authorized(holding_account(creator_holding_id(), funded))];
    for i in 0..n {
        let schedule = AccountId::new([100u8.saturating_add(i); 32]);
        pre.push(fresh_account(schedule, true));
        pre.push(fresh_account(
            compute_escrow_pda(program_id(), schedule),
            false,
        ));
    }
    pre
}

#[test]
#[ignore = "cycle benchmark: run explicitly with --ignored --nocapture"]
fn vesting_cycle_costs() {
    let create = run(
        &[
            fresh_account(schedule_id(), true),
            {
                let mut creator = holding_account(creator_holding_id(), TOTAL);
                creator.is_authorized = true;
                creator
            },
            fresh_account(escrow_id(), false),
        ],
        &params().into_create_instruction(),
    );
    report("CreateSchedule", &create);

    let claim = run(
        &[
            schedule_account_with(&cliff_linear_schedule()),
            holding_account(escrow_id(), TOTAL),
            authorized(holding_account(beneficiary_holding_id(), 0)),
            clock_account((START + END) / 2),
        ],
        &Instruction::Claim,
    );
    report("Claim", &claim);

    let claim_to = run(
        &[
            schedule_account_with(&cliff_linear_schedule()),
            holding_account(escrow_id(), TOTAL),
            authorized(holding_account(beneficiary_holding_id(), 0)),
            fresh_account(AccountId::new([26; 32]), true),
            clock_account((START + END) / 2),
        ],
        &Instruction::ClaimTo,
    );
    report("ClaimTo", &claim_to);

    let claim_to_private = run(
        &[
            schedule_account_with(&cliff_linear_schedule()),
            holding_account(escrow_id(), TOTAL),
            authorized(holding_account(beneficiary_holding_id(), 0)),
            fresh_account(AccountId::new([27; 32]), true),
        ],
        &Instruction::ClaimToPrivate {
            as_of: (START + END) / 2,
        },
    );
    report("ClaimToPrivate", &claim_to_private);

    let cancel = run(
        &[
            schedule_account_with(&cliff_linear_schedule()),
            holding_account(escrow_id(), TOTAL),
            holding_account(creator_holding_id(), 0),
            clock_account((START + END) / 2),
            authorized(holding_account(creator_holding_id(), 0)),
        ],
        &Instruction::Cancel,
    );
    report("Cancel", &cancel);

    let make_non_cancelable = run(
        &[
            schedule_account_with(&cliff_linear_schedule()),
            authorized(holding_account(creator_holding_id(), 0)),
        ],
        &Instruction::MakeNonCancelable,
    );
    report("MakeNonCancelable", &make_non_cancelable);

    let signal = run(
        &[
            schedule_account_with(&milestone_schedule()),
            authorized(holding_account(creator_holding_id(), 0)),
        ],
        &Instruction::SignalMilestone { index: 0 },
    );
    report("SignalMilestone", &signal);

    let transfer = run(
        &[
            schedule_account_with(&cliff_linear_schedule()),
            authorized(holding_account(beneficiary_holding_id(), 0)),
        ],
        &Instruction::TransferBeneficiary {
            new_beneficiary_holding_id: AccountId::new([25; 32]),
        },
    );
    report("TransferBeneficiary", &transfer);

    for n in [2u8, 10u8] {
        let batch = run(
            &batch_pre_states(n),
            &Instruction::CreateScheduleBatch {
                params: (0..n).map(|_| params()).collect(),
            },
        );
        report(&format!("Batch(n={n})"), &batch);
    }
}
