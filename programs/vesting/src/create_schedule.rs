use lee_core::{
    account::{Account, AccountId, AccountWithMetadata, Data},
    program::{AccountPostState, ChainedCall, Claim, ProgramId},
};
use token_core::TokenHolding;
use vesting_core::{
    compute_escrow_pda, compute_escrow_pda_seed, ScheduleParams, VestingEvent, VestingSchedule,
    MAX_BATCH_SCHEDULES,
};

/// Builds the initial schedule state for `params` (shared by single and batch
/// creation). Panics with a deterministic message on invalid parameters.
#[must_use]
pub fn build_schedule(
    creator_holding_id: AccountId,
    escrow_id: AccountId,
    params: &ScheduleParams,
) -> VestingSchedule {
    assert!(
        params.total_amount > 0,
        "Create schedule: amount must be nonzero"
    );
    params.kind.validate(params.total_amount);
    VestingSchedule {
        creator_holding_id,
        beneficiary_holding_id: params.beneficiary_holding_id,
        escrow_id,
        token_program_id: params.token_program_id,
        total_amount: params.total_amount,
        claimed_amount: 0,
        kind: params.kind.clone(),
        milestones_signalled: 0,
        cancelable: params.cancelable,
        transferable: params.transferable,
        cancelled_at: None,
        cancel_authority_id: params.cancel_authority_id.unwrap_or(creator_holding_id),
        milestone_authority_id: params.milestone_authority_id.unwrap_or(creator_holding_id),
        event_seq: 0,
        last_event: VestingEvent::Created {
            total_amount: params.total_amount,
        },
    }
}

/// Validates a (schedule target, escrow) pair for creation under `vesting_program_id`.
pub fn validate_schedule_pair(
    schedule_target: &AccountWithMetadata,
    escrow: &AccountWithMetadata,
    vesting_program_id: ProgramId,
) {
    assert_eq!(
        schedule_target.account,
        Account::default(),
        "Create schedule: schedule target account must have default values"
    );
    assert!(
        schedule_target.is_authorized,
        "Create schedule: schedule target account must be authorized"
    );
    assert_eq!(
        escrow.account_id,
        compute_escrow_pda(vesting_program_id, schedule_target.account_id),
        "Create schedule: escrow Account ID does not match PDA"
    );
    assert_eq!(
        escrow.account,
        Account::default(),
        "Create schedule: escrow account must be uninitialized"
    );
}

/// Builds the chained Token Program transfer funding `escrow` from
/// `creator_holding`. The escrow is a fresh PDA: it is marked authorized so
/// the Token Program accepts it as an uninitialized recipient; the runtime
/// validates that authorization against the attached PDA seed.
#[must_use]
pub fn fund_escrow_call(
    creator_holding: AccountWithMetadata,
    escrow: AccountWithMetadata,
    schedule_id: AccountId,
    amount: u128,
    token_program_id: ProgramId,
) -> ChainedCall {
    let mut escrow_auth = escrow;
    escrow_auth.is_authorized = true;
    ChainedCall::new(
        token_program_id,
        vec![creator_holding, escrow_auth],
        &token_core::Instruction::Transfer {
            amount_to_transfer: amount,
        },
    )
    .with_pda_seeds(vec![compute_escrow_pda_seed(schedule_id)])
}

/// Creates one schedule and escrows `params.total_amount` of the creator's
/// tokens in a program-owned vault.
///
/// The escrow vault is a PDA derived from the schedule account ID, so each
/// schedule owns an isolated vault. The token movement itself is delegated to
/// the Token Program via a chained call authorized with the vault's PDA seed.
pub fn create_schedule(
    schedule_target: AccountWithMetadata,
    creator_holding: AccountWithMetadata,
    escrow: AccountWithMetadata,
    params: ScheduleParams,
    vesting_program_id: ProgramId,
) -> (Vec<AccountPostState>, Vec<ChainedCall>) {
    assert!(
        creator_holding.is_authorized,
        "Create schedule: creator holding must be authorized"
    );
    validate_schedule_pair(&schedule_target, &escrow, vesting_program_id);

    let schedule = build_schedule(creator_holding.account_id, escrow.account_id, &params);
    let mut schedule_post = schedule_target.account;
    schedule_post.data = Data::from(&schedule);

    // Every declared account is echoed in the output (LEZ v0.2.4 rule).
    let creator_echo = creator_holding.account.clone();
    let escrow_echo = escrow.account.clone();
    let fund = fund_escrow_call(
        creator_holding,
        escrow,
        schedule_target.account_id,
        params.total_amount,
        params.token_program_id,
    );

    (
        vec![
            AccountPostState::new_claimed(schedule_post, Claim::Authorized),
            AccountPostState::new(creator_echo),
            AccountPostState::new(escrow_echo),
        ],
        vec![fund],
    )
}

/// Creates `params.len()` schedules in one transaction. `rest` holds the
/// (schedule target, escrow) pairs in order. The batch size is bounded by the
/// runtime's chained-call and compute limits; see README for the measured
/// maximum.
pub fn create_schedule_batch(
    creator_holding: AccountWithMetadata,
    rest: Vec<AccountWithMetadata>,
    params: Vec<ScheduleParams>,
    vesting_program_id: ProgramId,
) -> (Vec<AccountPostState>, Vec<ChainedCall>) {
    assert!(!params.is_empty(), "Create batch: batch must not be empty");
    assert!(
        params.len() <= MAX_BATCH_SCHEDULES,
        "Create batch: at most MAX_BATCH_SCHEDULES schedules per transaction"
    );
    assert!(
        creator_holding.is_authorized,
        "Create batch: creator holding must be authorized"
    );
    assert_eq!(
        rest.len(),
        params.len().checked_mul(2).expect("batch size fits usize"),
        "Create batch: expected one schedule/escrow pair per params entry"
    );

    let mut post_states = vec![AccountPostState::new(creator_holding.account.clone())];
    let mut chained = Vec::with_capacity(params.len());
    // Chained calls execute sequentially and each is validated against the
    // state left by the previous one, so the creator holding handed to call
    // `i` must already reflect the `i` earlier debits.
    let mut creator_running = creator_holding;
    for (pair, p) in rest.chunks_exact(2).zip(params.iter()) {
        let [schedule_target, escrow] = pair else {
            unreachable!("chunks_exact(2) yields pairs")
        };
        validate_schedule_pair(schedule_target, escrow, vesting_program_id);
        let schedule = build_schedule(creator_running.account_id, escrow.account_id, p);
        let mut schedule_post = schedule_target.account.clone();
        schedule_post.data = Data::from(&schedule);
        post_states.push(AccountPostState::new_claimed(
            schedule_post,
            Claim::Authorized,
        ));
        post_states.push(AccountPostState::new(escrow.account.clone()));
        chained.push(fund_escrow_call(
            creator_running.clone(),
            escrow.clone(),
            schedule_target.account_id,
            p.total_amount,
            p.token_program_id,
        ));
        creator_running = debit_holding(creator_running, p.total_amount);
    }
    (post_states, chained)
}

/// Returns `holding` with `amount` subtracted from its fungible balance —
/// the pre-state the next chained transfer will be validated against.
fn debit_holding(mut holding: AccountWithMetadata, amount: u128) -> AccountWithMetadata {
    let debited = match TokenHolding::try_from(&holding.account.data)
        .expect("Create batch: creator holding must be a token holding")
    {
        TokenHolding::Fungible {
            definition_id,
            balance,
        } => TokenHolding::Fungible {
            definition_id,
            balance: balance
                .checked_sub(amount)
                .expect("Create batch: creator holding balance cannot cover the batch"),
        },
        TokenHolding::NftMaster { .. } | TokenHolding::NftPrintedCopy { .. } => {
            panic!("Create batch: creator holding must be fungible")
        }
    };
    holding.account.data = Data::from(&debited);
    holding
}
