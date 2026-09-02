//! End-to-end tests for the Vesting Program (RFP-017).
//!
//! Validates the full lifecycle through the zkVM state machine: schedule
//! creation moves tokens into a program-derived escrow vault via a chained
//! Token Program transfer; beneficiary-signed claims (`Claim`, `ClaimTo`,
//! `ClaimToPrivate`) pay
//! vested tokens out of that vault; cancellation splits vested/unvested;
//! milestones unlock in order; transferable positions move; batch creation
//! is bisected for its maximum size.

use std::collections::HashMap;

use clock_core::{ClockAccountData, CLOCK_01_PROGRAM_ACCOUNT_ID};
use lee::{
    execute_and_prove,
    privacy_preserving_transaction::{
        circuit::ProgramWithDependencies, Message as PrivateMessage, PrivacyPreservingTransaction,
        WitnessSet as PrivateWitnessSet,
    },
    program::Program,
    program_deployment_transaction::{self, ProgramDeploymentTransaction},
    public_transaction, PrivateKey, PublicKey, PublicTransaction, V03State,
};
use lee_core::{
    account::{Account, AccountId, AccountWithMetadata, Data, Nonce},
    encryption::ViewingPublicKey,
    InputAccountIdentity, NullifierPublicKey, NullifierSecretKey,
};
use token_core::{TokenDefinition, TokenHolding};
use vesting_core::{compute_escrow_pda, ScheduleKind, ScheduleParams, VestingSchedule};

const TOTAL: u128 = 1_000;
const START: u64 = 10_000;
const CLIFF: u64 = 20_000;
const END: u64 = 30_000;

struct Keys;
struct Ids;
struct Accounts;

impl Keys {
    fn def_key() -> PrivateKey {
        PrivateKey::try_new([41; 32]).expect("valid private key")
    }

    fn creator_key() -> PrivateKey {
        PrivateKey::try_new([42; 32]).expect("valid private key")
    }

    fn beneficiary_key() -> PrivateKey {
        PrivateKey::try_new([43; 32]).expect("valid private key")
    }

    fn schedule_key() -> PrivateKey {
        PrivateKey::try_new([44; 32]).expect("valid private key")
    }

    fn cancel_auth_key() -> PrivateKey {
        PrivateKey::try_new([45; 32]).expect("valid private key")
    }

    fn milestone_auth_key() -> PrivateKey {
        PrivateKey::try_new([46; 32]).expect("valid private key")
    }

    fn beneficiary2_key() -> PrivateKey {
        PrivateKey::try_new([47; 32]).expect("valid private key")
    }

    fn batch_key(i: u8) -> PrivateKey {
        PrivateKey::try_new([200u8.saturating_add(i); 32]).expect("valid private key")
    }
}

fn id_of(key: &PrivateKey) -> AccountId {
    AccountId::from(&PublicKey::new_from_private_key(key))
}

impl Ids {
    fn token_program() -> lee_core::program::ProgramId {
        token_methods::TOKEN_ID
    }

    fn vesting_program() -> lee_core::program::ProgramId {
        vesting_methods::VESTING_ID
    }

    fn token_definition() -> AccountId {
        AccountId::from(&PublicKey::new_from_private_key(&Keys::def_key()))
    }

    fn creator_holding() -> AccountId {
        AccountId::from(&PublicKey::new_from_private_key(&Keys::creator_key()))
    }

    fn beneficiary_holding() -> AccountId {
        AccountId::from(&PublicKey::new_from_private_key(&Keys::beneficiary_key()))
    }

    fn schedule() -> AccountId {
        AccountId::from(&PublicKey::new_from_private_key(&Keys::schedule_key()))
    }

    fn escrow() -> AccountId {
        compute_escrow_pda(Ids::vesting_program(), Ids::schedule())
    }

    fn cancel_authority() -> AccountId {
        id_of(&Keys::cancel_auth_key())
    }

    fn milestone_authority() -> AccountId {
        id_of(&Keys::milestone_auth_key())
    }

    fn beneficiary2_holding() -> AccountId {
        id_of(&Keys::beneficiary2_key())
    }

    fn batch_schedule(i: u8) -> AccountId {
        id_of(&Keys::batch_key(i))
    }

    fn batch_escrow(i: u8) -> AccountId {
        compute_escrow_pda(Ids::vesting_program(), Ids::batch_schedule(i))
    }
}

impl Accounts {
    fn token_definition_init() -> Account {
        Account {
            program_owner: Ids::token_program(),
            balance: 0_u128,
            data: Data::from(&TokenDefinition::Fungible {
                name: String::from("Gold"),
                total_supply: TOTAL,
                metadata_id: None,
                authority: None,
            }),
            nonce: Nonce(0),
        }
    }

    fn holding_init(balance: u128) -> Account {
        Account {
            program_owner: Ids::token_program(),
            balance: 0_u128,
            data: Data::from(&TokenHolding::Fungible {
                definition_id: Ids::token_definition(),
                balance,
            }),
            nonce: Nonce(0),
        }
    }
}

fn advance_clock(state: &mut V03State, timestamp: u64) {
    let data = ClockAccountData {
        block_id: 0,
        timestamp,
    }
    .to_bytes();
    let clock_account = Account {
        // The real CLOCK_01 system account is owned by the clock program, not the
        // default program. A default owner makes the spel-framework output filter
        // drop the (unchanged, unclaimed) clock post-state, which the v0.2.1+
        // DeclaredAccountMissingFromOutput invariant then rejects. Use the same
        // non-default placeholder owner as the upstream AMM test fixtures.
        program_owner: [8u32; 8],
        data: Data::try_from(data).expect("clock account data fits"),
        ..Account::default()
    };
    state.force_insert_account(CLOCK_01_PROGRAM_ACCOUNT_ID, clock_account);
}

fn deploy(state: &mut V03State, elf: &[u8]) {
    let message = program_deployment_transaction::Message::new(elf.to_vec());
    let tx = ProgramDeploymentTransaction::new(message);
    state
        .transition_from_program_deployment_transaction(&tx)
        .expect("program deployment must succeed");
}

fn state_for_vesting_tests() -> V03State {
    let mut state = V03State::new();
    deploy(&mut state, token_methods::TOKEN_ELF);
    deploy(&mut state, vesting_methods::VESTING_ELF);
    state.force_insert_account(Ids::token_definition(), Accounts::token_definition_init());
    state.force_insert_account(Ids::creator_holding(), Accounts::holding_init(TOTAL));
    state.force_insert_account(Ids::beneficiary_holding(), Accounts::holding_init(0));
    state.force_insert_account(Ids::beneficiary2_holding(), Accounts::holding_init(0));
    // Authorities are token holdings (program-owned accounts). A plain,
    // never-used wallet account works as an authority exactly once: after its
    // first signed transaction its nonce is non-zero, the spel-framework
    // output filter drops it from the program output (it only keeps
    // default-owned accounts that are still in their default state), and LEZ
    // v0.2.4 then rejects the transaction with DeclaredAccountMissingFromOutput.
    // See `plain_wallet_authority_can_sign_only_once` below.
    state.force_insert_account(Ids::cancel_authority(), Accounts::holding_init(0));
    state.force_insert_account(Ids::milestone_authority(), Accounts::holding_init(0));
    advance_clock(&mut state, START);
    state
}

/// Sends a public transaction signed by `signers`; nonces are read from the
/// current state so helpers stay correct across multiple signed transactions.
fn send(
    state: &mut V03State,
    accounts: Vec<AccountId>,
    signers: &[&PrivateKey],
    instruction: vesting_core::Instruction,
) -> Result<(), lee::error::LeeError> {
    let nonces: Vec<Nonce> = signers
        .iter()
        .map(|key| state.get_account_by_id(id_of(key)).nonce)
        .collect();
    let message =
        public_transaction::Message::try_new(Ids::vesting_program(), accounts, nonces, instruction)
            .expect("message must build");
    let witness_set = public_transaction::WitnessSet::for_message(&message, signers);
    let tx = PublicTransaction::new(message, witness_set);
    state
        .transition_from_public_transaction(&tx, 0, 0)
        .map(drop)
}

fn default_params() -> ScheduleParams {
    ScheduleParams {
        beneficiary_holding_id: Ids::beneficiary_holding(),
        total_amount: TOTAL,
        kind: ScheduleKind::CliffLinear {
            start: START,
            cliff: CLIFF,
            end: END,
        },
        cancelable: true,
        transferable: false,
        cancel_authority_id: Some(Ids::cancel_authority()),
        milestone_authority_id: Some(Ids::milestone_authority()),
        token_program_id: Ids::token_program(),
    }
}

fn execute_create_schedule_with(state: &mut V03State, params: ScheduleParams) {
    send(
        state,
        vec![Ids::schedule(), Ids::creator_holding(), Ids::escrow()],
        &[&Keys::schedule_key(), &Keys::creator_key()],
        params.into_create_instruction(),
    )
    .expect("create-schedule transaction must succeed");
}

fn execute_create_schedule(state: &mut V03State) {
    execute_create_schedule_with(state, default_params());
}

/// Claim signed by `beneficiary_key`, whose holding must be the schedule's
/// registered beneficiary.
fn try_execute_claim_signed(
    state: &mut V03State,
    beneficiary_key: &PrivateKey,
) -> Result<(), lee::error::LeeError> {
    send(
        state,
        vec![
            Ids::schedule(),
            Ids::escrow(),
            id_of(beneficiary_key),
            CLOCK_01_PROGRAM_ACCOUNT_ID,
        ],
        &[beneficiary_key],
        vesting_core::Instruction::Claim,
    )
}

fn try_execute_claim(state: &mut V03State) -> Result<(), lee::error::LeeError> {
    try_execute_claim_signed(state, &Keys::beneficiary_key())
}

/// Unsigned claim: must be rejected (the beneficiary decides when to claim).
fn try_execute_claim_unsigned(
    state: &mut V03State,
    beneficiary_holding: AccountId,
) -> Result<(), lee::error::LeeError> {
    send(
        state,
        vec![
            Ids::schedule(),
            Ids::escrow(),
            beneficiary_holding,
            CLOCK_01_PROGRAM_ACCOUNT_ID,
        ],
        &[],
        vesting_core::Instruction::Claim,
    )
}

fn try_execute_cancel(
    state: &mut V03State,
    signers: &[&PrivateKey],
) -> Result<(), lee::error::LeeError> {
    send(
        state,
        vec![
            Ids::schedule(),
            Ids::escrow(),
            Ids::creator_holding(),
            CLOCK_01_PROGRAM_ACCOUNT_ID,
            Ids::cancel_authority(),
        ],
        signers,
        vesting_core::Instruction::Cancel,
    )
}

fn try_signal_milestone(state: &mut V03State, index: u32) -> Result<(), lee::error::LeeError> {
    send(
        state,
        vec![Ids::schedule(), Ids::milestone_authority()],
        &[&Keys::milestone_auth_key()],
        vesting_core::Instruction::SignalMilestone { index },
    )
}

fn holding_balance(state: &V03State, id: AccountId) -> u128 {
    match TokenHolding::try_from(&state.get_account_by_id(id).data)
        .expect("holding data must parse")
    {
        TokenHolding::Fungible { balance, .. } => balance,
        TokenHolding::NftMaster { .. } | TokenHolding::NftPrintedCopy { .. } => {
            panic!("expected fungible holding")
        }
    }
}

#[test]
fn create_schedule_escrows_tokens_in_program_vault() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);

    // Creator paid the full amount into the program-derived escrow vault.
    assert_eq!(holding_balance(&state, Ids::creator_holding()), 0);
    assert_eq!(holding_balance(&state, Ids::escrow()), TOTAL);

    // Schedule account is owned by the vesting program and records the terms.
    let schedule_account = state.get_account_by_id(Ids::schedule());
    assert_eq!(schedule_account.program_owner, Ids::vesting_program());
    let schedule =
        VestingSchedule::try_from(&schedule_account.data).expect("schedule data must parse");
    assert_eq!(schedule.total_amount, TOTAL);
    assert_eq!(schedule.claimed_amount, 0);
    assert_eq!(schedule.escrow_id, Ids::escrow());
}

#[test]
fn claim_before_cliff_is_rejected() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);

    advance_clock(&mut state, CLIFF - 1);
    assert!(try_execute_claim(&mut state).is_err());

    // Nothing moved.
    assert_eq!(holding_balance(&state, Ids::escrow()), TOTAL);
    assert_eq!(holding_balance(&state, Ids::beneficiary_holding()), 0);
}

#[test]
fn unsigned_claim_is_rejected_so_nobody_can_force_a_public_claim() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);
    advance_clock(&mut state, END + 1);
    assert!(try_execute_claim_unsigned(&mut state, Ids::beneficiary_holding()).is_err());
    assert_eq!(
        holding_balance(&state, Ids::escrow()),
        TOTAL,
        "nothing moved"
    );
}

#[test]
fn claim_to_a_second_public_holding_is_authorised_by_the_beneficiary() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);
    advance_clock(&mut state, END + 1);
    send(
        &mut state,
        vec![
            Ids::schedule(),
            Ids::escrow(),
            Ids::beneficiary_holding(),
            Ids::beneficiary2_holding(),
            CLOCK_01_PROGRAM_ACCOUNT_ID,
        ],
        &[&Keys::beneficiary_key()],
        vesting_core::Instruction::ClaimTo,
    )
    .expect("claim_to must succeed");
    assert_eq!(holding_balance(&state, Ids::beneficiary2_holding()), TOTAL);
    assert_eq!(holding_balance(&state, Ids::beneficiary_holding()), 0);
}

#[test]
fn claim_pays_pro_rata_and_remainder_at_end() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);

    // Midpoint of [START, END]: half of the total is vested.
    advance_clock(&mut state, (START + END) / 2);
    try_execute_claim(&mut state).expect("mid-schedule claim must succeed");

    assert_eq!(
        holding_balance(&state, Ids::beneficiary_holding()),
        TOTAL / 2
    );
    assert_eq!(holding_balance(&state, Ids::escrow()), TOTAL - TOTAL / 2);
    let schedule = VestingSchedule::try_from(&state.get_account_by_id(Ids::schedule()).data)
        .expect("schedule data must parse");
    assert_eq!(schedule.claimed_amount, TOTAL / 2);

    // After the end: the remainder is claimable, and nothing more after that.
    advance_clock(&mut state, END + 1);
    try_execute_claim(&mut state).expect("post-end claim must succeed");

    assert_eq!(holding_balance(&state, Ids::beneficiary_holding()), TOTAL);
    assert_eq!(holding_balance(&state, Ids::escrow()), 0);

    assert!(
        try_execute_claim(&mut state).is_err(),
        "fully claimed schedule must reject further claims"
    );
}

// ── Cancellation ───────────────────────────────────────────────────────

#[test]
fn cancel_splits_vested_and_unvested() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);
    advance_clock(&mut state, (START + END) / 2);
    try_execute_cancel(&mut state, &[&Keys::cancel_auth_key()]).expect("cancel must succeed");

    assert_eq!(
        holding_balance(&state, Ids::creator_holding()),
        TOTAL / 2,
        "unvested returned to creator"
    );
    assert_eq!(
        holding_balance(&state, Ids::escrow()),
        TOTAL / 2,
        "vested stays escrowed"
    );

    // The beneficiary can still claim what had vested, and nothing more later.
    advance_clock(&mut state, END + 1);
    try_execute_claim(&mut state).expect("post-cancel claim of the vested part must succeed");
    assert_eq!(
        holding_balance(&state, Ids::beneficiary_holding()),
        TOTAL / 2
    );
    assert!(
        try_execute_claim(&mut state).is_err(),
        "nothing left after cancellation"
    );
    assert_eq!(holding_balance(&state, Ids::escrow()), 0);
}

#[test]
fn cancel_is_rejected_without_authority_signature() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);
    assert!(try_execute_cancel(&mut state, &[]).is_err());
    assert_eq!(
        holding_balance(&state, Ids::escrow()),
        TOTAL,
        "nothing moved"
    );
    assert_eq!(holding_balance(&state, Ids::creator_holding()), 0);
}

#[test]
fn make_non_cancelable_blocks_cancellation_for_good() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);
    send(
        &mut state,
        vec![Ids::schedule(), Ids::cancel_authority()],
        &[&Keys::cancel_auth_key()],
        vesting_core::Instruction::MakeNonCancelable,
    )
    .expect("make_non_cancelable must succeed");
    let schedule = VestingSchedule::try_from(&state.get_account_by_id(Ids::schedule()).data)
        .expect("schedule data must parse");
    assert!(!schedule.cancelable);

    advance_clock(&mut state, (START + END) / 2);
    assert!(try_execute_cancel(&mut state, &[&Keys::cancel_auth_key()]).is_err());
    assert_eq!(
        holding_balance(&state, Ids::escrow()),
        TOTAL,
        "escrow untouched"
    );
}

// ── Milestones ─────────────────────────────────────────────────────────

#[test]
fn milestone_schedule_unlocks_per_signal_and_rejects_double_signal() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule_with(
        &mut state,
        ScheduleParams {
            kind: ScheduleKind::Milestone {
                tranches: vec![TOTAL / 4, TOTAL - TOTAL / 4],
            },
            ..default_params()
        },
    );
    assert!(
        try_execute_claim(&mut state).is_err(),
        "nothing unlocked yet"
    );

    try_signal_milestone(&mut state, 0).expect("first signal");
    try_execute_claim(&mut state).expect("claim first tranche");
    assert_eq!(
        holding_balance(&state, Ids::beneficiary_holding()),
        TOTAL / 4
    );

    assert!(
        try_signal_milestone(&mut state, 0).is_err(),
        "double signal rejected"
    );
    assert!(
        try_execute_claim(&mut state).is_err(),
        "no double unlock: nothing new to claim"
    );
    assert_eq!(holding_balance(&state, Ids::escrow()), TOTAL - TOTAL / 4);

    try_signal_milestone(&mut state, 1).expect("second signal");
    try_execute_claim(&mut state).expect("claim remainder");
    assert_eq!(holding_balance(&state, Ids::beneficiary_holding()), TOTAL);
    assert_eq!(holding_balance(&state, Ids::escrow()), 0);
}

// ── Transferable positions ─────────────────────────────────────────────

#[test]
fn transferable_position_moves_to_new_beneficiary() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule_with(
        &mut state,
        ScheduleParams {
            transferable: true,
            ..default_params()
        },
    );
    send(
        &mut state,
        vec![Ids::schedule(), Ids::beneficiary_holding()],
        &[&Keys::beneficiary_key()],
        vesting_core::Instruction::TransferBeneficiary {
            new_beneficiary_holding_id: Ids::beneficiary2_holding(),
        },
    )
    .expect("transfer must succeed");

    advance_clock(&mut state, END + 1);
    assert!(
        try_execute_claim_signed(&mut state, &Keys::beneficiary_key()).is_err(),
        "old beneficiary no longer matches the schedule"
    );
    try_execute_claim_signed(&mut state, &Keys::beneficiary2_key())
        .expect("new beneficiary claims");
    assert_eq!(holding_balance(&state, Ids::beneficiary2_holding()), TOTAL);
    assert_eq!(holding_balance(&state, Ids::beneficiary_holding()), 0);
}

#[test]
fn non_transferable_position_rejects_transfer() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);
    assert!(send(
        &mut state,
        vec![Ids::schedule(), Ids::beneficiary_holding()],
        &[&Keys::beneficiary_key()],
        vesting_core::Instruction::TransferBeneficiary {
            new_beneficiary_holding_id: Ids::beneficiary2_holding(),
        },
    )
    .is_err());
}

// ── Batch creation ─────────────────────────────────────────────────────

fn state_for_batch_tests(n: u8) -> V03State {
    let mut state = V03State::new();
    deploy(&mut state, token_methods::TOKEN_ELF);
    deploy(&mut state, vesting_methods::VESTING_ELF);
    state.force_insert_account(Ids::token_definition(), Accounts::token_definition_init());
    let funded = TOTAL
        .checked_mul(u128::from(n))
        .expect("batch funding fits u128");
    state.force_insert_account(Ids::creator_holding(), Accounts::holding_init(funded));
    state.force_insert_account(Ids::beneficiary_holding(), Accounts::holding_init(0));
    advance_clock(&mut state, START);
    state
}

fn execute_create_batch(state: &mut V03State, n: u8) -> Result<(), lee::error::LeeError> {
    let mut accounts = vec![Ids::creator_holding()];
    for i in 0..n {
        accounts.push(Ids::batch_schedule(i));
        accounts.push(Ids::batch_escrow(i));
    }
    let keys: Vec<PrivateKey> = (0..n).map(Keys::batch_key).collect();
    let mut signers: Vec<&PrivateKey> = Vec::with_capacity(keys.len().saturating_add(1));
    let creator_key = Keys::creator_key();
    signers.push(&creator_key);
    signers.extend(keys.iter());
    let params = (0..n).map(|_| default_params()).collect();
    send(
        state,
        accounts,
        &signers,
        vesting_core::Instruction::CreateScheduleBatch { params },
    )
}

#[test]
fn batch_create_max_size_is_documented() {
    let mut max_ok = 0u8;
    for n in 1u8..=16 {
        let mut state = state_for_batch_tests(n);
        match execute_create_batch(&mut state, n) {
            Ok(()) => {
                for i in 0..n {
                    assert_eq!(holding_balance(&state, Ids::batch_escrow(i)), TOTAL);
                }
                assert_eq!(holding_balance(&state, Ids::creator_holding()), 0);
                max_ok = n;
            }
            Err(err) => {
                eprintln!("batch of {n} rejected: {err:?}");
                break;
            }
        }
    }
    eprintln!("MAX_BATCH_SCHEDULES = {max_ok}");
    assert_eq!(
        usize::from(max_ok),
        vesting_core::MAX_BATCH_SCHEDULES,
        "the runtime's chained-call limit must match the enforced batch maximum"
    );
}

// ── Platform constraint: plain wallet accounts as authorities ──────────

/// Documents a spel-framework/LEZ v0.2.4 interaction: a default-owned wallet
/// account that has already signed once (nonce > 0) is dropped from the
/// program output by the spel-framework filter, and the runtime then rejects
/// the transaction because a declared account is missing from the output.
/// Program-owned accounts (token holdings) are not affected, which is why the
/// vesting program expects authorities to be holdings.
#[test]
fn plain_wallet_authority_can_sign_only_once() {
    let mut state = state_for_vesting_tests();
    // Replace the milestone authority with a plain, never-used wallet account.
    state.force_insert_account(Ids::milestone_authority(), Account::default());
    execute_create_schedule_with(
        &mut state,
        ScheduleParams {
            kind: ScheduleKind::Milestone {
                tranches: vec![TOTAL / 4, TOTAL - TOTAL / 4],
            },
            ..default_params()
        },
    );

    try_signal_milestone(&mut state, 0).expect("first signal from a fresh wallet account works");
    let second = try_signal_milestone(&mut state, 1);
    assert!(
        matches!(
            second,
            Err(lee::error::LeeError::InvalidProgramBehavior(
                lee::error::InvalidProgramBehaviorError::DeclaredAccountMissingFromOutput { .. }
            ))
        ),
        "second signal from the same plain wallet account is rejected by the output filter: {second:?}"
    );
}

// ── Private claim path (research test) ─────────────────────────────────

/// Claims to a fresh private account chosen at claim time: the registered
/// (public) beneficiary holding signs a `ClaimTo` whose destination is a
/// `PrivateForeignInit` — a new private account nobody has spent from — and
/// the chained Token Program transfer credits it. Observers see the claim
/// amount and the schedule; the destination note and its later movements are
/// shielded, and the creator never learns the private account.
#[test]
fn claim_to_fresh_private_account() {
    let mut state = state_for_vesting_tests();

    // Private destination key material (same construction as the ATA test).
    let destination_nsk: NullifierSecretKey = [13u8; 32];
    let destination_npk = NullifierPublicKey::from(&destination_nsk);
    let destination_vpk = ViewingPublicKey::from_seed(&[31u8; 32], &[32u8; 32]);
    let private_destination =
        AccountId::for_regular_private_account(&destination_npk, &destination_vpk, 0);

    execute_create_schedule(&mut state);
    advance_clock(&mut state, END + 1);

    let schedule_pre = AccountWithMetadata::new(
        state.get_account_by_id(Ids::schedule()),
        false,
        Ids::schedule(),
    );
    let escrow_pre =
        AccountWithMetadata::new(state.get_account_by_id(Ids::escrow()), false, Ids::escrow());
    // The registered beneficiary holding authorises the claim (public signature).
    let beneficiary_pre = AccountWithMetadata::new(
        state.get_account_by_id(Ids::beneficiary_holding()),
        true,
        Ids::beneficiary_holding(),
    );
    // Fresh private account: authorized (it is the claimant's own new note).
    let destination_pre = AccountWithMetadata::new(Account::default(), true, private_destination);
    let clock_pre = AccountWithMetadata::new(
        state.get_account_by_id(CLOCK_01_PROGRAM_ACCOUNT_ID),
        false,
        CLOCK_01_PROGRAM_ACCOUNT_ID,
    );

    let instruction_data = Program::serialize_instruction(vesting_core::Instruction::ClaimTo)
        .expect("instruction serializes");
    let commitment_root = state.commitment_root();

    let vesting_program =
        Program::new(vesting_methods::VESTING_ELF.to_vec().into()).expect("vesting program");
    let token_program =
        Program::new(token_methods::TOKEN_ELF.to_vec().into()).expect("token program");
    let program_with_deps = ProgramWithDependencies::new(
        vesting_program,
        HashMap::from([(Ids::token_program(), token_program)]),
    );

    let (output, proof) = execute_and_prove(
        vec![
            schedule_pre,
            escrow_pre,
            beneficiary_pre,
            destination_pre,
            clock_pre,
        ],
        instruction_data,
        vec![
            InputAccountIdentity::Public,
            InputAccountIdentity::Public,
            InputAccountIdentity::Public,
            InputAccountIdentity::PrivateForeignInit {
                vpk: destination_vpk,
                random_seed: [0; 32],
                npk: destination_npk,
                identifier: 0,
                commitment_root,
            },
            InputAccountIdentity::Public,
        ],
        &program_with_deps,
    )
    .expect("private claim executes and proves");

    let beneficiary_nonce = state.get_account_by_id(Ids::beneficiary_holding()).nonce;
    let message = PrivateMessage::from_circuit_output(vec![beneficiary_nonce], output);
    let witness_set = PrivateWitnessSet::for_message(&message, proof, &[&Keys::beneficiary_key()]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);
    state
        .transition_from_privacy_preserving_transaction(&tx, 0, 0)
        .expect("private claim transaction applies");

    // The escrow paid out in full to the private destination; the registered
    // public holding received nothing; the schedule records the claim.
    assert_eq!(holding_balance(&state, Ids::escrow()), 0);
    assert_eq!(holding_balance(&state, Ids::beneficiary_holding()), 0);
    let schedule = VestingSchedule::try_from(&state.get_account_by_id(Ids::schedule()).data)
        .expect("schedule data must parse");
    assert_eq!(schedule.claimed_amount, TOTAL);
}

/// Builds one `ClaimToPrivate { as_of }` transaction against the current
/// state: schedule, escrow, signing beneficiary holding, and a fresh private
/// destination (deterministic key material). No clock account is declared.
fn build_private_claim_tx(state: &V03State, as_of: u64) -> PrivacyPreservingTransaction {
    let destination_nsk: NullifierSecretKey = [17u8; 32];
    let destination_npk = NullifierPublicKey::from(&destination_nsk);
    let destination_vpk = ViewingPublicKey::from_seed(&[33u8; 32], &[34u8; 32]);
    let private_destination =
        AccountId::for_regular_private_account(&destination_npk, &destination_vpk, 0);

    let schedule_pre = AccountWithMetadata::new(
        state.get_account_by_id(Ids::schedule()),
        false,
        Ids::schedule(),
    );
    let escrow_pre =
        AccountWithMetadata::new(state.get_account_by_id(Ids::escrow()), false, Ids::escrow());
    let beneficiary_pre = AccountWithMetadata::new(
        state.get_account_by_id(Ids::beneficiary_holding()),
        true,
        Ids::beneficiary_holding(),
    );
    let destination_pre = AccountWithMetadata::new(Account::default(), true, private_destination);

    let instruction_data =
        Program::serialize_instruction(vesting_core::Instruction::ClaimToPrivate { as_of })
            .expect("instruction serializes");
    let commitment_root = state.commitment_root();
    let vesting_program =
        Program::new(vesting_methods::VESTING_ELF.to_vec().into()).expect("vesting program");
    let token_program =
        Program::new(token_methods::TOKEN_ELF.to_vec().into()).expect("token program");
    let program_with_deps = ProgramWithDependencies::new(
        vesting_program,
        HashMap::from([(Ids::token_program(), token_program)]),
    );

    let (output, proof) = execute_and_prove(
        vec![schedule_pre, escrow_pre, beneficiary_pre, destination_pre],
        instruction_data,
        vec![
            InputAccountIdentity::Public,
            InputAccountIdentity::Public,
            InputAccountIdentity::Public,
            InputAccountIdentity::PrivateForeignInit {
                vpk: destination_vpk,
                random_seed: [0; 32],
                npk: destination_npk,
                identifier: 0,
                commitment_root,
            },
        ],
        &program_with_deps,
    )
    .expect("private claim executes and proves without a clock account");

    let beneficiary_nonce = state.get_account_by_id(Ids::beneficiary_holding()).nonce;
    let message = PrivateMessage::from_circuit_output(vec![beneficiary_nonce], output);
    let witness_set = PrivateWitnessSet::for_message(&message, proof, &[&Keys::beneficiary_key()]);
    PrivacyPreservingTransaction::new(message, witness_set)
}

/// `ClaimToPrivate { as_of }` declares no clock account, so the proof binds
/// only the schedule, the vault and the beneficiary holding; the runtime
/// enforces `as_of` as the lower bound of the timestamp validity window.
#[test]
fn private_claim_pins_validity_window_without_clock() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);
    // The clock is deliberately left at its fixture value: the private claim
    // must not read it.
    let as_of = END + 1;
    let tx = build_private_claim_tx(&state, as_of);

    // Included "before" as_of: rejected by the validity window.
    let early = state.transition_from_privacy_preserving_transaction(&tx, 1, as_of - 1);
    assert!(
        matches!(early, Err(lee::error::LeeError::OutOfValidityWindow)),
        "inclusion before as_of must be rejected, got {early:?}"
    );
    assert_eq!(
        holding_balance(&state, Ids::escrow()),
        TOTAL,
        "nothing moved"
    );

    // Included at/after as_of: applies; escrow pays out in full.
    state
        .transition_from_privacy_preserving_transaction(&tx, 2, as_of)
        .expect("private claim applies at as_of");
    assert_eq!(holding_balance(&state, Ids::escrow()), 0);
    assert_eq!(holding_balance(&state, Ids::beneficiary_holding()), 0);
    let schedule = VestingSchedule::try_from(&state.get_account_by_id(Ids::schedule()).data)
        .expect("schedule data must parse");
    assert_eq!(schedule.claimed_amount, TOTAL);
}

/// A private claim is single-use: re-submitting the identical transaction
/// after inclusion is rejected. The runtime checks the beneficiary's nonce
/// first (it advanced with the first inclusion); the schedule pre-state and
/// the destination's init nullifier would independently fail behind it.
#[test]
fn private_claim_replay_is_rejected() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);
    let as_of = END + 1;
    let tx = build_private_claim_tx(&state, as_of);

    state
        .transition_from_privacy_preserving_transaction(&tx, 1, as_of)
        .expect("first private claim applies");
    assert_eq!(holding_balance(&state, Ids::escrow()), 0);

    let replay = state.transition_from_privacy_preserving_transaction(&tx, 2, as_of + 1);
    assert!(
        matches!(&replay, Err(lee::error::LeeError::InvalidInput(msg)) if msg == "Nonce mismatch"),
        "replaying an included private claim must be rejected, got {replay:?}"
    );

    assert_eq!(holding_balance(&state, Ids::escrow()), 0);
    assert_eq!(holding_balance(&state, Ids::beneficiary_holding()), 0);
    let schedule = VestingSchedule::try_from(&state.get_account_by_id(Ids::schedule()).data)
        .expect("schedule data must parse");
    assert_eq!(schedule.claimed_amount, TOTAL, "nothing claimed twice");
}

/// The amount paid is what was vested at `as_of`, not at the inclusion
/// timestamp: a claim proved at the schedule midpoint and included after the
/// end still pays exactly half. The remainder stays claimable publicly.
#[test]
fn private_claim_pays_vested_at_as_of_not_at_inclusion() {
    let mut state = state_for_vesting_tests();
    execute_create_schedule(&mut state);
    let as_of = (START + END) / 2;
    let tx = build_private_claim_tx(&state, as_of);

    // Included long after the schedule fully vested.
    state
        .transition_from_privacy_preserving_transaction(&tx, 1, END + 1_000)
        .expect("private claim applies after as_of");
    assert_eq!(holding_balance(&state, Ids::escrow()), TOTAL / 2);
    assert_eq!(holding_balance(&state, Ids::beneficiary_holding()), 0);
    let schedule = VestingSchedule::try_from(&state.get_account_by_id(Ids::schedule()).data)
        .expect("schedule data must parse");
    assert_eq!(
        schedule.claimed_amount,
        TOTAL / 2,
        "paid what was vested at as_of"
    );

    // The other half is still owed and a public claim collects it.
    advance_clock(&mut state, END + 2_000);
    try_execute_claim(&mut state).expect("public claim of the remainder must succeed");
    assert_eq!(holding_balance(&state, Ids::escrow()), 0);
    assert_eq!(
        holding_balance(&state, Ids::beneficiary_holding()),
        TOTAL / 2
    );
    let schedule = VestingSchedule::try_from(&state.get_account_by_id(Ids::schedule()).data)
        .expect("schedule data must parse");
    assert_eq!(schedule.claimed_amount, TOTAL);
}
