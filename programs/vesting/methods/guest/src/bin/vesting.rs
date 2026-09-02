#![cfg_attr(not(test), no_main)]
#![allow(
    clippy::cloned_ref_to_slice_refs,
    reason = "SPEL macro emits cloned validation slices for one-account instructions"
)]

use nssa_core::account::{AccountId, AccountWithMetadata};
use nssa_core::program::ProgramId;
use spel_framework::context::ProgramContext;
use spel_framework::prelude::*;

#[cfg(not(test))]
risc0_zkvm::guest::entry!(main);

#[lez_program(instruction = "vesting_core::Instruction")]
mod vesting {
    #[expect(
        unused_imports,
        reason = "SPEL instruction macro requires importing parent-scope handler types"
    )]
    use super::*;

    /// Create a vesting schedule; the total amount moves into a
    /// program-owned escrow vault via a chained Token Program transfer.
    /// `tranches` non-empty selects a milestone schedule (start/cliff/end
    /// ignored); otherwise cliff+linear, with `cliff == start` for fully linear.
    #[expect(
        clippy::too_many_arguments,
        reason = "flattened schedule parameters keep the instruction callable from the SPEL CLI"
    )]
    #[instruction]
    pub fn create_schedule(
        ctx: ProgramContext,
        #[account(init, signer)]
        schedule_target: AccountWithMetadata,
        #[account(mut, signer)]
        creator_holding: AccountWithMetadata,
        #[account(mut)]
        escrow: AccountWithMetadata,
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
    ) -> SpelResult {
        let params = vesting_core::ScheduleParams::from_flat(
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
        let (post_states, chained_calls) = vesting_program::create_schedule::create_schedule(
            schedule_target,
            creator_holding,
            escrow,
            params,
            ctx.self_program_id,
        );
        Ok(spel_framework::SpelOutput::execute(
            post_states,
            chained_calls,
        ))
    }

    /// Create several schedules at once; `rest` = [schedule_0, escrow_0, schedule_1, escrow_1, …].
    #[instruction]
    pub fn create_schedule_batch(
        ctx: ProgramContext,
        #[account(mut, signer)]
        creator_holding: AccountWithMetadata,
        #[account(mut)]
        rest: Vec<AccountWithMetadata>,
        params: Vec<vesting_core::ScheduleParams>,
    ) -> SpelResult {
        let (post_states, chained_calls) =
            vesting_program::create_schedule::create_schedule_batch(
                creator_holding,
                rest,
                params,
                ctx.self_program_id,
            );
        Ok(spel_framework::SpelOutput::execute(
            post_states,
            chained_calls,
        ))
    }

    /// Claim all currently vested, unclaimed tokens to the beneficiary holding.
    /// Signed by the beneficiary: tokens only move escrow -> beneficiary.
    #[instruction]
    pub fn claim(
        #[account(mut)]
        schedule: AccountWithMetadata,
        #[account(mut)]
        escrow: AccountWithMetadata,
        #[account(mut, signer)]
        beneficiary_holding: AccountWithMetadata,
        clock: AccountWithMetadata,
    ) -> SpelResult {
        let (post_states, chained_calls) =
            vesting_program::claim::claim(schedule, escrow, beneficiary_holding, clock);
        Ok(spel_framework::SpelOutput::execute(
            post_states,
            chained_calls,
        ))
    }

    /// Claim to an explicit destination (public holding or fresh private
    /// account), authorised by the beneficiary holding's signature.
    #[instruction]
    pub fn claim_to(
        #[account(mut)]
        schedule: AccountWithMetadata,
        #[account(mut)]
        escrow: AccountWithMetadata,
        #[account(signer)]
        beneficiary_holding: AccountWithMetadata,
        #[account(mut)]
        destination: AccountWithMetadata,
        clock: AccountWithMetadata,
    ) -> SpelResult {
        let (post_states, chained_calls) = vesting_program::claim::claim_to(
            schedule,
            escrow,
            beneficiary_holding,
            destination,
            clock,
        );
        Ok(spel_framework::SpelOutput::execute(
            post_states,
            chained_calls,
        ))
    }

    /// Cancel a cancelable schedule; unvested tokens return to the creator,
    /// vested-but-unclaimed tokens stay claimable.
    #[instruction]
    pub fn cancel(
        #[account(mut)]
        schedule: AccountWithMetadata,
        #[account(mut)]
        escrow: AccountWithMetadata,
        #[account(mut)]
        creator_holding: AccountWithMetadata,
        clock: AccountWithMetadata,
        #[account(signer)]
        cancel_authority: AccountWithMetadata,
    ) -> SpelResult {
        let (post_states, chained_calls) = vesting_program::cancel::cancel(
            schedule,
            escrow,
            creator_holding,
            clock,
            cancel_authority,
        );
        Ok(spel_framework::SpelOutput::execute(
            post_states,
            chained_calls,
        ))
    }

    /// One-way conversion of a cancelable schedule to non-cancelable.
    #[instruction]
    pub fn make_non_cancelable(
        #[account(mut)]
        schedule: AccountWithMetadata,
        #[account(signer)]
        cancel_authority: AccountWithMetadata,
    ) -> SpelResult {
        let (post_states, chained_calls) =
            vesting_program::cancel::make_non_cancelable(schedule, cancel_authority);
        Ok(spel_framework::SpelOutput::execute(
            post_states,
            chained_calls,
        ))
    }

    /// Unlock the next milestone tranche (milestone schedules only).
    #[instruction]
    pub fn signal_milestone(
        #[account(mut)]
        schedule: AccountWithMetadata,
        #[account(signer)]
        milestone_authority: AccountWithMetadata,
        index: u32,
    ) -> SpelResult {
        let (post_states, chained_calls) =
            vesting_program::milestone::signal_milestone(schedule, milestone_authority, index);
        Ok(spel_framework::SpelOutput::execute(
            post_states,
            chained_calls,
        ))
    }

    /// Move a transferable position to a new beneficiary holding.
    #[instruction]
    pub fn transfer_beneficiary(
        #[account(mut)]
        schedule: AccountWithMetadata,
        #[account(signer)]
        beneficiary_holding: AccountWithMetadata,
        new_beneficiary_holding_id: AccountId,
    ) -> SpelResult {
        let (post_states, chained_calls) = vesting_program::transfer::transfer_beneficiary(
            schedule,
            beneficiary_holding,
            new_beneficiary_holding_id,
        );
        Ok(spel_framework::SpelOutput::execute(
            post_states,
            chained_calls,
        ))
    }

    /// Private claim without a clock account: pays what is vested at `as_of`
    /// to a fresh private `destination` and pins the output's timestamp
    /// validity window to `as_of..` (enforced by the runtime at inclusion).
    #[instruction]
    pub fn claim_to_private(
        #[account(mut)]
        schedule: AccountWithMetadata,
        #[account(mut)]
        escrow: AccountWithMetadata,
        #[account(signer)]
        beneficiary_holding: AccountWithMetadata,
        #[account(mut)]
        destination: AccountWithMetadata,
        as_of: u64,
    ) -> SpelResult {
        let (post_states, chained_calls, window) = vesting_program::claim::claim_to_private(
            schedule,
            escrow,
            beneficiary_holding,
            destination,
            as_of,
        );
        let mut output = spel_framework::SpelOutput::execute(post_states, chained_calls);
        output.timestamp_validity_window = window;
        Ok(output)
    }
}
