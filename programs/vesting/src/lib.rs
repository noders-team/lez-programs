//! The Vesting Program implementation (RFP-017).
//!
//! Cliff+linear, fully linear and milestone-based token vesting with escrow
//! held in a program-derived vault and claims paid out via chained calls to
//! the Token Program.

pub use vesting_core as core;

pub mod cancel;
pub mod claim;
pub mod create_schedule;
pub mod milestone;
pub mod transfer;

mod tests;
