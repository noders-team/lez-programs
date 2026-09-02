//! Core data structures and vesting math for the Vesting Program.
//!
//! RFP-017 (Privacy-Preserving Token Vesting): cliff+linear, fully linear and
//! milestone-based schedules with token escrow held in a program-derived
//! account (PDA) and claims paid out via chained calls to the Token Program.

use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::{
    account::{AccountId, Data},
    program::{PdaSeed, ProgramId},
    Timestamp,
};
use serde::{Deserialize, Serialize};
use spel_framework_macros::account_type;

/// Domain separator for escrow vault PDA derivation.
pub const ESCROW_PDA_DOMAIN: &[u8] = b"VESTING_ESCROW";

/// Maximum schedules per `CreateScheduleBatch`. Each schedule needs one
/// chained Token Program transfer and the LEZ runtime allows at most 10
/// chained calls per transaction (`MAX_NUMBER_CHAINED_CALLS`); the handler
/// enforces this bound so the documented maximum cannot drift silently.
pub const MAX_BATCH_SCHEDULES: usize = 10;

/// Vesting Program Instruction.
///
/// Account lists are documented per instruction; the order is the ABI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    /// Create one schedule and escrow `total_amount` from the creator.
    ///
    /// The schedule parameters are flattened so the instruction is callable
    /// from the SPEL CLI (which encodes primitives, `Option`, `Vec` and
    /// program ids, but not nested structs). `tranches` non-empty selects a
    /// milestone schedule and `start`/`cliff`/`end` are ignored; otherwise a
    /// cliff+linear schedule (`cliff == start` for fully linear).
    ///
    /// Accounts: schedule target (uninitialized, signer), creator holding
    /// (signer), escrow vault (uninitialized PDA of the schedule target).
    CreateSchedule {
        beneficiary_holding_id: AccountId,
        total_amount: u128,
        start: u64,
        cliff: u64,
        end: u64,
        tranches: Vec<u128>,
        cancelable: bool,
        transferable: bool,
        cancel_authority_id: Option<AccountId>,
        milestone_authority_id: Option<AccountId>,
        token_program_id: ProgramId,
    },

    /// Create `params.len()` schedules in one transaction.
    ///
    /// Accounts: creator holding (signer), then for each schedule `i`:
    /// schedule target `i` (uninitialized, signer), escrow vault `i` (PDA).
    CreateScheduleBatch { params: Vec<ScheduleParams> },

    /// Pay all currently claimable tokens to the beneficiary holding.
    ///
    /// Signed by the beneficiary holding: only the beneficiary decides when
    /// and where vested tokens surface (a third party cannot force a public
    /// claim and defeat a later private one), and the signature nonce makes
    /// every claim transaction unique.
    ///
    /// Accounts: schedule, escrow, beneficiary holding (signer), clock.
    Claim,

    /// Pay all currently claimable tokens to `destination` instead of the
    /// registered beneficiary holding — a public holding or a fresh private
    /// account (the private claim path). Signed by the beneficiary holding.
    ///
    /// Accounts: schedule, escrow, beneficiary holding (signer), destination,
    /// clock.
    ClaimTo,

    /// Cancel a cancelable schedule: unvested tokens return to the creator
    /// holding; vested-but-unclaimed tokens stay claimable.
    ///
    /// Accounts: schedule, escrow, creator holding, clock, cancellation
    /// authority (signer; equals the creator holding unless nominated).
    Cancel,

    /// Irreversibly convert a cancelable schedule to non-cancelable.
    ///
    /// Accounts: schedule, cancellation authority (signer).
    MakeNonCancelable,

    /// Unlock milestone `index` of a milestone schedule. Indices must be
    /// signalled in order; re-signalling is rejected.
    ///
    /// Accounts: schedule, milestone authority (signer).
    SignalMilestone { index: u32 },

    /// Move the beneficiary position to a new holding (transferable only).
    ///
    /// Accounts: schedule, current beneficiary holding (signer).
    TransferBeneficiary {
        new_beneficiary_holding_id: AccountId,
    },

    /// Private claim: pay all tokens vested at `as_of` to `destination` — a
    /// fresh private account — without reading the clock. The program pins
    /// `timestamp_validity_window = as_of..`, so the runtime rejects inclusion
    /// before `as_of` and the amount paid is never more than what is vested at
    /// inclusion time. Signed by the beneficiary holding.
    ///
    /// Accounts: schedule, escrow, beneficiary holding (signer), destination
    /// (fresh private account).
    ClaimToPrivate { as_of: u64 },
}

/// Creation-time parameters of a schedule (shared by single and batch create).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleParams {
    pub beneficiary_holding_id: AccountId,
    pub total_amount: u128,
    pub kind: ScheduleKind,
    pub cancelable: bool,
    pub transferable: bool,
    /// `None` = the creator holding cancels.
    pub cancel_authority_id: Option<AccountId>,
    /// `None` = the creator holding signals milestones.
    pub milestone_authority_id: Option<AccountId>,
    /// Token Program the escrow transfers are chained to.
    pub token_program_id: ProgramId,
}

impl ScheduleParams {
    /// Rebuilds the parameters from the flattened `CreateSchedule` fields.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the flattened CLI-facing instruction fields"
    )]
    pub fn from_flat(
        beneficiary_holding_id: AccountId,
        total_amount: u128,
        start: u64,
        cliff: u64,
        end: u64,
        tranches: Vec<u128>,
        cancelable: bool,
        transferable: bool,
        cancel_authority_id: Option<AccountId>,
        milestone_authority_id: Option<AccountId>,
        token_program_id: ProgramId,
    ) -> Self {
        let kind = if tranches.is_empty() {
            ScheduleKind::CliffLinear { start, cliff, end }
        } else {
            ScheduleKind::Milestone { tranches }
        };
        Self {
            beneficiary_holding_id,
            total_amount,
            kind,
            cancelable,
            transferable,
            cancel_authority_id,
            milestone_authority_id,
            token_program_id,
        }
    }

    /// Flattens the parameters into a `CreateSchedule` instruction.
    #[must_use]
    pub fn into_create_instruction(self) -> Instruction {
        let (start, cliff, end, tranches) = match self.kind {
            ScheduleKind::CliffLinear { start, cliff, end } => (start, cliff, end, Vec::new()),
            ScheduleKind::Milestone { tranches } => (0, 0, 0, tranches),
        };
        Instruction::CreateSchedule {
            beneficiary_holding_id: self.beneficiary_holding_id,
            total_amount: self.total_amount,
            start,
            cliff,
            end,
            tranches,
            cancelable: self.cancelable,
            transferable: self.transferable,
            cancel_authority_id: self.cancel_authority_id,
            milestone_authority_id: self.milestone_authority_id,
            token_program_id: self.token_program_id,
        }
    }
}

/// The three RFP-017 schedule types. Fully-linear vesting is `CliffLinear`
/// with `cliff == start`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ScheduleKind {
    CliffLinear {
        /// Vesting start (ms since the Unix epoch, as the LEZ clock reports).
        /// Accrual is measured from here.
        start: u64,
        /// Cliff (ms): nothing is claimable before this point.
        cliff: u64,
        /// Vesting end (ms): everything is claimable from this point.
        end: u64,
    },
    Milestone {
        /// Fixed unlock amounts, in signalling order; must sum to the total.
        tranches: Vec<u128>,
    },
}

impl ScheduleKind {
    /// Panics with a deterministic message when the parameters are invalid.
    pub fn validate(&self, total_amount: u128) {
        match self {
            ScheduleKind::CliffLinear { start, cliff, end } => {
                assert!(
                    start <= cliff && cliff <= end,
                    "Create schedule: require start <= cliff <= end"
                );
                assert!(end > start, "Create schedule: duration must be nonzero");
            }
            ScheduleKind::Milestone { tranches } => {
                assert!(
                    !tranches.is_empty(),
                    "Create schedule: tranches must be non-empty"
                );
                let mut sum: u128 = 0;
                for tranche in tranches {
                    assert!(
                        *tranche > 0,
                        "Create schedule: tranche amounts must be nonzero"
                    );
                    sum = sum
                        .checked_add(*tranche)
                        .expect("Create schedule: tranche sum overflows u128");
                }
                assert_eq!(
                    sum, total_amount,
                    "Create schedule: tranches must sum to total_amount"
                );
            }
        }
    }
}

/// Event record written into the schedule on every state transition.
///
/// This is the event seam: LEZ has no runtime event/log API yet (LP-0012 is
/// awarded but not integrated into the runtime). Indexers read
/// `event_seq`/`last_event` from the schedule account per transaction; when
/// the runtime mechanism lands, the same `VestingEvent` is emitted through it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum VestingEvent {
    Created {
        total_amount: u128,
    },
    Claimed {
        amount: u128,
    },
    Cancelled {
        returned: u128,
    },
    MadeNonCancelable,
    MilestoneSignalled {
        index: u32,
        amount: u128,
    },
    BeneficiaryTransferred {
        new_beneficiary_holding_id: AccountId,
    },
}

/// State of a single vesting schedule.
#[account_type]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct VestingSchedule {
    /// Token Holding account of the schedule creator (unvested tokens return
    /// here on cancellation).
    pub creator_holding_id: AccountId,
    /// Token Holding account the vested tokens are claimable to.
    pub beneficiary_holding_id: AccountId,
    /// Escrow vault (PDA of the Vesting Program) holding the locked tokens.
    pub escrow_id: AccountId,
    /// Program ID of the Token Program used for escrow transfers.
    pub token_program_id: ProgramId,
    /// Total amount locked at creation.
    pub total_amount: u128,
    /// Amount already claimed by the beneficiary.
    pub claimed_amount: u128,
    pub kind: ScheduleKind,
    /// Number of milestone tranches unlocked so far (milestone schedules).
    pub milestones_signalled: u32,
    pub cancelable: bool,
    pub transferable: bool,
    /// Set once by `Cancel`; freezes accrual at that instant (ms).
    pub cancelled_at: Option<u64>,
    pub cancel_authority_id: AccountId,
    pub milestone_authority_id: AccountId,
    /// Monotonic per-schedule event counter (event seam).
    pub event_seq: u64,
    pub last_event: VestingEvent,
}

impl TryFrom<&Data> for VestingSchedule {
    type Error = std::io::Error;

    fn try_from(data: &Data) -> Result<Self, Self::Error> {
        VestingSchedule::try_from_slice(data.as_ref())
    }
}

impl From<&VestingSchedule> for Data {
    fn from(schedule: &VestingSchedule) -> Self {
        let mut data = Vec::with_capacity(std::mem::size_of_val(schedule));
        BorshSerialize::serialize(schedule, &mut data)
            .expect("Serialization to Vec should not fail");
        Data::try_from(data).expect("Vesting schedule encoded data should fit into Data")
    }
}

impl VestingSchedule {
    /// Amount vested (claimable + already claimed) at time `now`.
    ///
    /// After cancellation, accrual is frozen at `cancelled_at`.
    #[must_use]
    pub fn vested_at(&self, now: Timestamp) -> u128 {
        let effective_now = match self.cancelled_at {
            Some(cancelled_at) if cancelled_at < now => cancelled_at,
            _ => now,
        };
        match &self.kind {
            ScheduleKind::CliffLinear { start, cliff, end } => {
                Self::cliff_linear_vested(self.total_amount, *start, *cliff, *end, effective_now)
            }
            ScheduleKind::Milestone { tranches } => {
                let signalled =
                    usize::try_from(self.milestones_signalled).expect("milestone count fits usize");
                tranches.iter().take(signalled).fold(0u128, |acc, tranche| {
                    acc.checked_add(*tranche)
                        .expect("vested_at: tranche sum cannot overflow")
                })
            }
        }
    }

    /// Cliff + linear: zero before the cliff; at the cliff the tranche accrued
    /// since `start` unlocks at once; linear accrual until `end`; the full
    /// amount after `end`.
    fn cliff_linear_vested(
        total_amount: u128,
        start: Timestamp,
        cliff: Timestamp,
        end: Timestamp,
        now: Timestamp,
    ) -> u128 {
        if now < cliff {
            return 0;
        }
        if now >= end {
            return total_amount;
        }
        // now is in [cliff, end) and end > start is enforced at creation.
        let elapsed = u128::from(now.saturating_sub(start));
        let duration = u128::from(end.saturating_sub(start));
        // total_amount * elapsed / duration, floor, without 256-bit widening:
        // split total into (q * duration + r). Both partial products fit in
        // u128 because elapsed < duration < 2^64 and q * elapsed <= total.
        let q = total_amount
            .checked_div(duration)
            .expect("vested_at: duration is non-zero because end > start");
        let r = total_amount
            .checked_rem(duration)
            .expect("vested_at: duration is non-zero because end > start");
        let whole = q
            .checked_mul(elapsed)
            .expect("vested_at: whole-part product cannot overflow");
        let frac = r
            .checked_mul(elapsed)
            .expect("vested_at: fractional product cannot overflow")
            .checked_div(duration)
            .expect("vested_at: duration is non-zero because end > start");
        whole
            .checked_add(frac)
            .expect("vested_at: vested amount cannot exceed total")
    }

    /// Amount claimable now (vested minus already claimed).
    #[must_use]
    pub fn claimable_at(&self, now: Timestamp) -> u128 {
        self.vested_at(now).saturating_sub(self.claimed_amount)
    }

    /// Records `event` as the latest transition and bumps the sequence.
    #[must_use]
    pub fn with_event(self, event: VestingEvent) -> Self {
        Self {
            event_seq: self
                .event_seq
                .checked_add(1)
                .expect("event sequence cannot overflow"),
            last_event: event,
            ..self
        }
    }
}

/// Derives the [`PdaSeed`] of the escrow vault for a given schedule account.
#[must_use]
pub fn compute_escrow_pda_seed(schedule_id: AccountId) -> PdaSeed {
    use risc0_zkvm::sha::{Impl, Sha256};

    let mut bytes = Vec::with_capacity(ESCROW_PDA_DOMAIN.len().saturating_add(32));
    bytes.extend_from_slice(ESCROW_PDA_DOMAIN);
    bytes.extend_from_slice(schedule_id.value());
    PdaSeed::new(
        Impl::hash_bytes(&bytes)
            .as_bytes()
            .try_into()
            .expect("Hash output must be exactly 32 bytes long"),
    )
}

/// Derives the escrow vault [`AccountId`] for a schedule account.
#[must_use]
pub fn compute_escrow_pda(vesting_program_id: ProgramId, schedule_id: AccountId) -> AccountId {
    AccountId::for_public_pda(&vesting_program_id, &compute_escrow_pda_seed(schedule_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(kind: ScheduleKind, total: u128) -> VestingSchedule {
        VestingSchedule {
            creator_holding_id: AccountId::new([1; 32]),
            beneficiary_holding_id: AccountId::new([2; 32]),
            escrow_id: AccountId::new([3; 32]),
            token_program_id: [0u32; 8],
            total_amount: total,
            claimed_amount: 0,
            kind,
            milestones_signalled: 0,
            cancelable: true,
            transferable: false,
            cancelled_at: None,
            cancel_authority_id: AccountId::new([1; 32]),
            milestone_authority_id: AccountId::new([1; 32]),
            event_seq: 0,
            last_event: VestingEvent::Created {
                total_amount: total,
            },
        }
    }

    fn cliff_linear(total: u128, start: u64, cliff: u64, end: u64) -> VestingSchedule {
        base(ScheduleKind::CliffLinear { start, cliff, end }, total)
    }

    #[test]
    fn nothing_vests_before_cliff() {
        let s = cliff_linear(1_000, 0, 500, 1_000);
        assert_eq!(s.vested_at(0), 0);
        assert_eq!(s.vested_at(499), 0);
    }

    #[test]
    fn cliff_unlocks_accrued_tranche() {
        assert_eq!(cliff_linear(1_000, 0, 500, 1_000).vested_at(500), 500);
    }

    #[test]
    fn linear_between_cliff_and_end() {
        assert_eq!(cliff_linear(1_000, 0, 250, 1_000).vested_at(750), 750);
    }

    #[test]
    fn everything_vests_at_end() {
        let s = cliff_linear(1_000, 0, 500, 1_000);
        assert_eq!(s.vested_at(1_000), 1_000);
        assert_eq!(s.vested_at(u64::MAX), 1_000);
    }

    #[test]
    fn fully_linear_when_cliff_equals_start() {
        let s = cliff_linear(1_000, 1_000, 1_000, 2_000);
        assert_eq!(s.vested_at(1_000), 0);
        assert_eq!(s.vested_at(1_500), 500);
    }

    #[test]
    fn claimable_subtracts_claimed() {
        let mut s = cliff_linear(1_000, 0, 0, 1_000);
        s.claimed_amount = 300;
        assert_eq!(s.claimable_at(500), 200);
        assert_eq!(s.claimable_at(200), 0);
    }

    #[test]
    fn no_overflow_on_large_amounts() {
        let s = cliff_linear(u128::MAX / 2, 0, 0, u64::MAX);
        let v = s.vested_at(u64::MAX / 2);
        assert!(v > 0 && v < u128::MAX / 2);
    }

    #[test]
    fn cancellation_freezes_vesting_at_cancel_time() {
        let mut s = cliff_linear(1_000, 0, 0, 1_000);
        s.cancelled_at = Some(400);
        assert_eq!(s.vested_at(400), 400);
        assert_eq!(s.vested_at(900), 400, "nothing accrues after cancellation");
        assert_eq!(s.vested_at(100), 100, "history before cancel is unchanged");
    }

    #[test]
    fn milestone_vests_sum_of_signalled_tranches() {
        let mut s = base(
            ScheduleKind::Milestone {
                tranches: vec![100, 300, 600],
            },
            1_000,
        );
        assert_eq!(s.vested_at(0), 0);
        s.milestones_signalled = 1;
        assert_eq!(s.vested_at(0), 100);
        s.milestones_signalled = 3;
        assert_eq!(s.vested_at(u64::MAX), 1_000);
    }

    #[test]
    fn milestone_kind_validates_tranche_sum() {
        ScheduleKind::Milestone {
            tranches: vec![400, 600],
        }
        .validate(1_000);
    }

    #[test]
    #[should_panic(expected = "tranches must sum to total_amount")]
    fn milestone_kind_rejects_wrong_sum() {
        ScheduleKind::Milestone {
            tranches: vec![400, 500],
        }
        .validate(1_000);
    }

    #[test]
    #[should_panic(expected = "tranche amounts must be nonzero")]
    fn milestone_kind_rejects_zero_tranche() {
        ScheduleKind::Milestone {
            tranches: vec![0, 1_000],
        }
        .validate(1_000);
    }

    #[test]
    #[should_panic(expected = "require start <= cliff <= end")]
    fn cliff_linear_kind_rejects_bad_order() {
        ScheduleKind::CliffLinear {
            start: 10,
            cliff: 5,
            end: 20,
        }
        .validate(1);
    }

    #[test]
    fn with_event_increments_sequence() {
        let s = cliff_linear(1, 0, 0, 1).with_event(VestingEvent::Claimed { amount: 1 });
        assert_eq!(s.event_seq, 1);
        assert_eq!(s.last_event, VestingEvent::Claimed { amount: 1 });
    }

    #[test]
    fn flat_create_instruction_round_trips_params() {
        let milestone = ScheduleParams {
            beneficiary_holding_id: AccountId::new([2; 32]),
            total_amount: 3,
            kind: ScheduleKind::Milestone {
                tranches: vec![1, 2],
            },
            cancelable: false,
            transferable: true,
            cancel_authority_id: Some(AccountId::new([4; 32])),
            milestone_authority_id: None,
            token_program_id: [9u32; 8],
        };
        let Instruction::CreateSchedule {
            beneficiary_holding_id,
            total_amount,
            start,
            cliff,
            end,
            tranches,
            cancelable,
            transferable,
            cancel_authority_id,
            milestone_authority_id,
            token_program_id,
        } = milestone.clone().into_create_instruction()
        else {
            panic!("expected CreateSchedule")
        };
        let rebuilt = ScheduleParams::from_flat(
            beneficiary_holding_id,
            total_amount,
            start,
            cliff,
            end,
            tranches,
            cancelable,
            transferable,
            cancel_authority_id,
            milestone_authority_id,
            token_program_id,
        );
        assert_eq!(rebuilt, milestone);
    }

    /// The SPEL CLI encodes an instruction's enum discriminant from its position
    /// in the IDL, and the IDL lists instructions in guest-source order. The
    /// guest entries must therefore be declared in the same order as the
    /// `Instruction` variants, or the CLI silently dispatches the wrong variant.
    #[test]
    fn idl_instruction_order_matches_enum_discriminants() {
        let idl: serde_json::Value =
            serde_json::from_str(include_str!("../../../../artifacts/vesting-idl.json"))
                .expect("IDL parses");
        let idl_order: Vec<String> = idl["instructions"]
            .as_array()
            .expect("instructions array")
            .iter()
            .map(|ix| ix["name"].as_str().expect("name").to_owned())
            .collect();
        let id = AccountId::new([1; 32]);
        let samples: Vec<(&str, Instruction)> = vec![
            (
                "create_schedule",
                Instruction::CreateSchedule {
                    beneficiary_holding_id: id,
                    total_amount: 1,
                    start: 0,
                    cliff: 0,
                    end: 1,
                    tranches: vec![],
                    cancelable: true,
                    transferable: false,
                    cancel_authority_id: None,
                    milestone_authority_id: None,
                    token_program_id: [0u32; 8],
                },
            ),
            (
                "create_schedule_batch",
                Instruction::CreateScheduleBatch { params: vec![] },
            ),
            ("claim", Instruction::Claim),
            ("claim_to", Instruction::ClaimTo),
            ("cancel", Instruction::Cancel),
            ("make_non_cancelable", Instruction::MakeNonCancelable),
            (
                "signal_milestone",
                Instruction::SignalMilestone { index: 0 },
            ),
            (
                "transfer_beneficiary",
                Instruction::TransferBeneficiary {
                    new_beneficiary_holding_id: id,
                },
            ),
            ("claim_to_private", Instruction::ClaimToPrivate { as_of: 1 }),
        ];
        for (name, instruction) in samples {
            let words = risc0_zkvm::serde::to_vec(&instruction).expect("serializes");
            let discriminant = usize::try_from(words[0]).expect("fits");
            assert_eq!(
                idl_order.get(discriminant).map(String::as_str),
                Some(name),
                "IDL position of `{name}` must equal its enum discriminant {discriminant}"
            );
        }
    }

    #[test]
    fn schedule_round_trips_through_data() {
        let s = base(
            ScheduleKind::Milestone {
                tranches: vec![1, 2],
            },
            3,
        );
        let data = Data::from(&s);
        assert_eq!(VestingSchedule::try_from(&data).expect("decodes"), s);
    }
}
