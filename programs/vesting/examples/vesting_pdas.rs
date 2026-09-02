//! Print the escrow vault PDA(s) of vesting schedule account(s) for a deployment.
//!
//! Usage:
//!   cargo run -q -p vesting_program --example vesting_pdas -- <vesting_pid> <schedule_id>
//! [<schedule_id>...]
//!
//! `vesting_pid` is the ProgramId as 8 comma-separated u32 limbs, a 64-char ImageID hex, or a
//! base58 ImageID (as printed by `spel program-id`); `schedule_id`s are base58 account ids.
//! Every schedule owns one escrow vault: `AccountId::for_public_pda(vesting_pid,
//! sha256(b"VESTING_ESCROW" || schedule_id))`.

use std::str::FromStr;

use lee_core::{account::AccountId, program::ProgramId};
use vesting_core::compute_escrow_pda;

// Accepts a ProgramId as 8 comma-separated u32 limbs, a 64-char ImageID hex, or a base58
// ImageID. Hex/base58 are decoded as the 32 ImageID bytes read little-endian per u32 word,
// matching how `spel program-id` maps the ImageID to limbs.
fn parse_pid(s: &str) -> ProgramId {
    if s.contains(',') {
        let limbs: Vec<u32> = s
            .split(',')
            .map(|x| x.trim().parse().expect("ProgramId limb must be a u32"))
            .collect();
        assert_eq!(limbs.len(), 8, "ProgramId must be 8 u32 limbs");
        let mut pid: ProgramId = [0u32; 8];
        pid.copy_from_slice(&limbs);
        return pid;
    }
    let bytes: [u8; 32] = if s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        let mut out = [0u8; 32];
        for (byte, pair) in out.iter_mut().zip(s.as_bytes().chunks_exact(2)) {
            let pair: [u8; 2] = pair.try_into().expect("hex pair");
            let hex = std::str::from_utf8(&pair).expect("ascii hex");
            *byte = u8::from_str_radix(hex, 16).expect("invalid hex digit");
        }
        out
    } else {
        AccountId::from_str(s)
            .expect("ProgramId must be 8 u32 limbs, a 64-char hex ImageID, or base58")
            .into_value()
    };
    let mut pid: ProgramId = [0u32; 8];
    for (limb, chunk) in pid.iter_mut().zip(bytes.chunks_exact(4)) {
        *limb = u32::from_le_bytes(chunk.try_into().expect("4-byte chunk"));
    }
    pid
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((pid_s, schedules)) = args.split_first() else {
        eprintln!("usage: vesting_pdas <vesting_pid> <schedule_id> [<schedule_id>...]");
        std::process::exit(1);
    };
    if schedules.is_empty() {
        eprintln!("usage: vesting_pdas <vesting_pid> <schedule_id> [<schedule_id>...]");
        std::process::exit(1);
    }
    let vesting = parse_pid(pid_s);
    for schedule_s in schedules {
        let schedule = AccountId::from_str(schedule_s).expect("schedule_id must be base58");
        println!(
            "{schedule}  escrow  {}",
            compute_escrow_pda(vesting, schedule)
        );
    }
}
