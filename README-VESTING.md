# Vesting Program — RFP-017 Reference Implementation

An implementation of [RFP-017 — Privacy-Preserving Token Vesting](https://github.com/logos-co/rfp/blob/master/RFPs/RFP-017-token-vesting.md)
for the Logos Execution Zone, built by [NODERS](https://noders.team) on top of
upstream [`logos-blockchain/lez-programs`](https://github.com/logos-blockchain/lez-programs).
All changes are additive: a new `programs/vesting` crate family, an
integration test suite, and a cycle benchmark.

**Live on testnet 0.2.** ImageID
`952b031fecf9e357c76daca7c307e0debb4949d30d419b3c3a1db402a1fa46be` (program v4), deploy tx
[`5feaa17a…`](https://explorer.testnet.lez.logos.co/transaction/5feaa17ac617bf682cadd55e332fdbcdc8e4b727ade9fdc196f7da058528f775)
(block 27324, 2026-08-28 11:41 UTC). Full lifecycle on chain — 21 steps (15 included public
transactions, 5 rejected, 1 privacy-preserving claim), final balances reconciled — in
[`docs/vesting/testnet-evidence.md`](docs/vesting/testnet-evidence.md) (run 4); re-check
everything against the sequencer with `scripts/vesting/verify-onchain.sh` (ALL GOOD). Runbook:
[`docs/vesting/README.md`](docs/vesting/README.md).

**Platform baseline:** `lez-programs` main `6b53f3a` (2026-09-01), LEZ
`v0.2.4` (`lee`/`lee_core`), `spel-framework` rev `1ef0500f`, RISC Zero 3.0.5.
`CreateSchedule` takes flattened parameters so 7 of the 9 instructions are
callable from the `spel` CLI (see `docs/vesting/README.md`); the two exceptions
are `CreateScheduleBatch` (nested `Vec<ScheduleParams>`, SDK only) and
`ClaimToPrivate` (needs a privacy-preserving transaction, which `spel-cli`
cannot build — hence `tools/vesting-private-claim`).

## What it delivers

Every on-chain hard requirement of RFP-017, proven end-to-end through the
zkVM state machine:

- **Program-owned, token-denominated escrow.** Each schedule owns an isolated
  escrow vault: a PDA derived from the schedule account ID. Funding and
  payouts are chained `Transfer` calls into the Token Program, authorized via
  `ChainedCall::pda_seeds` — the custody pattern the upstream AMM uses for its
  pool vaults. No new token-program primitive is required.
- **Three schedule types.** Cliff + linear, fully linear (cliff = start), and
  milestone-based (creator-signalled tranches), all under one `ScheduleKind`.
- **Cancellation with the universal invariant.** Unvested tokens return to the
  creator; tokens vested up to the cancellation instant stay claimable.
  One-way `MakeNonCancelable` conversion.
- **Transferable positions** as a creation-time flag; the current beneficiary
  assigns the position to a new holding.
- **Separate cancellation and milestone authorities** (both soft
  requirements), nominated at creation.
- **Batch creation** — up to **10 schedules per transaction**. The LEZ
  transaction size limit (`max_block_size`, 1 MiB by default, minus the
  200 B block header — sequencer `service.rs`) would admit thousands of
  schedules; the binding limit on v0.2.4 is `MAX_NUMBER_CHAINED_CALLS = 10`
  (one chained escrow-funding transfer per schedule).
- **Event seam** — every state transition records a typed `VestingEvent` in
  the schedule (`event_seq` + `last_event`), ready to be re-emitted through
  the LEZ event mechanism when it lands in the runtime.
- **Claims to public or private accounts, chosen at claim time.** `Claim`
  pays the registered beneficiary holding; `ClaimTo` pays any other public
  holding the beneficiary names; `ClaimToPrivate { as_of }` pays a fresh
  private account. The private path runs as a privacy-preserving transaction
  whose destination is a private account initialised in the same transaction
  (`PrivateAuthorizedInit` under the beneficiary's own wallet key — what the
  tool and the testnet run use; the zkVM test `claim_to_fresh_private_account`
  uses `PrivateForeignInit`), credited by the same chained Token Program
  transfer, and declares no clock account: the program computes the vested
  amount at `as_of` and pins the output's `timestamp_validity_window` to
  `as_of..`, which the runtime checks at inclusion against the block
  timestamp. The proof still binds the current pre-state of the schedule, the
  escrow vault and the signing beneficiary holding, so any transaction that
  touches one of them before inclusion (a cancel, a milestone signal, another
  claim, any token transfer into the beneficiary holding) invalidates the
  pending proof and the claim is simply re-proved; and `as_of` must not be in
  the sequencer's future, or the transaction is dropped rather than queued.
  **Ran on the public testnet** — tx
  `3a2828ab85dcc503959447eb4a8734eaa4ab309fc6ef7f84869727c53b22c72b` (block `27362`). What stays
  hidden is *which* account (key) holds the tokens and their later movements;
  the opening balance equals the public claim amount (1,000) and is therefore
  inferable, and the destination account id is withheld from the evidence —
  the note is identified by its initialisation nullifier instead. See
  *Platform findings*.
- **Beneficiary-authorised claims.** Every claim is signed by the registered
  beneficiary holding: nobody can force a public claim and defeat a later
  private one, tokens only ever move from the escrow vault to a destination
  the beneficiary chose, and the signature nonce makes every claim
  transaction unique.

## Layout

```
programs/vesting/
  core/                 # VestingSchedule, ScheduleKind, ScheduleParams, VestingEvent,
                        # Instruction, vesting math, PDA helpers
  src/                  # Pure handlers: create_schedule (+ batch), claim, cancel
                        # (+ make_non_cancelable), milestone, transfer
  methods/guest/        # SPEL guest binary (#[lez_program] / #[instruction])
programs/integration_tests/tests/vesting.rs      # e2e through the zkVM state machine
programs/benchmark/tests/vesting_cycle_bench.rs  # executor cycle measurements
```

## Instruction reference

| Instruction | Declared accounts (order is the ABI) | Signers | Effect |
|---|---|---|---|
| `CreateSchedule { …flattened schedule params }` | schedule target (uninit), creator holding, escrow PDA (uninit) | schedule target, creator holding | Writes `VestingSchedule`; chained `Transfer(total)` creator → escrow |
| `CreateScheduleBatch { params[] }` | creator holding, then `(schedule_i, escrow_i)` pairs | creator holding, every schedule target | One schedule + one chained transfer per pair; max 10 |
| `Claim` | schedule, escrow, beneficiary holding, clock | beneficiary holding | Pays `claimable_at(now)`; chained `Transfer` escrow → beneficiary |
| `ClaimTo` | schedule, escrow, beneficiary holding, destination, clock | beneficiary holding | Pays `claimable_at(now)` to `destination` (another public holding) |
| `ClaimToPrivate { as_of }` | schedule, escrow, beneficiary holding, destination (fresh private account) | beneficiary holding | Pays `claimable_at(as_of)` to `destination`; no clock account — pins `timestamp_validity_window = as_of..`, enforced by the runtime at inclusion (`OutOfValidityWindow` before `as_of`); sent as a privacy-preserving transaction |
| `Cancel` | schedule, escrow, creator holding, clock, cancellation authority | cancellation authority | Freezes accrual at `now`; chained `Transfer(unvested)` escrow → creator |
| `MakeNonCancelable` | schedule, cancellation authority | cancellation authority | Sets `cancelable = false`, irreversible |
| `SignalMilestone { index }` | schedule, milestone authority | milestone authority | Unlocks tranche `index`; must equal the next unsignalled index |
| `TransferBeneficiary { new_beneficiary_holding_id }` | schedule, current beneficiary holding | current beneficiary | Reassigns the position (transferable schedules only) |

Every handler returns one post-state per declared account, in declaration
order; read-only accounts are echoed unchanged (LEZ v0.2.4 requires every
declared account in the program output).

### Schedule state

```rust
pub struct VestingSchedule {
    creator_holding_id, beneficiary_holding_id, escrow_id: AccountId,
    token_program_id: ProgramId,
    total_amount, claimed_amount: u128,
    kind: ScheduleKind,               // CliffLinear { start, cliff, end } | Milestone { tranches }
    milestones_signalled: u32,
    cancelable, transferable: bool,
    cancelled_at: Option<Timestamp>,  // accrual frozen here after Cancel
    cancel_authority_id, milestone_authority_id: AccountId,
    event_seq: u64,
    last_event: VestingEvent,
}
```

`vested_at(now)` = cliff+linear pro-rata (floor, no 256-bit widening) or the
sum of signalled tranches, evaluated at `min(now, cancelled_at)`.
`claimable_at(now) = vested_at(now) − claimed_amount`.

*Cliff interpretation.* Nothing is claimable before `cliff`; at `cliff` the
lump released is the amount accrued linearly since `start`, and the remainder
accrues linearly to `end`. For the RFP's "1-year cliff, 3-year linear release"
this is the appendix's four-year schedule: 25 % at the cliff, 75 % over the
following three years. An explicit `cliff_amount` (lump independent of the
accrual, remainder linear to `end`) is a planned M1 addition; fully linear is
`cliff == start`.

### Event schema (F6)

| `VestingEvent` | Emitted by |
|---|---|
| `Created { total_amount }` | `CreateSchedule`, `CreateScheduleBatch` |
| `Claimed { amount }` | `Claim`, `ClaimTo`, `ClaimToPrivate` |
| `Cancelled { returned }` | `Cancel` |
| `MadeNonCancelable` | `MakeNonCancelable` |
| `MilestoneSignalled { index, amount }` | `SignalMilestone` |
| `BeneficiaryTransferred { new_beneficiary_holding_id }` | `TransferBeneficiary` |

`event_seq` increments on every transition, so an indexer reading the
schedule account per transaction reconstructs the full event stream. LEZ
v0.2.4 has no runtime event API (LP-0012); upstream `dev` merged
`feat!: events` on 2026-08-27 (unreleased), and the seam maps onto its
`ProgramEvent { selector, data }` — the same `VestingEvent` values are
emitted through it once it ships.

## Platform findings

- **Custody needs no LP-0013 primitive.** LP-0013 is closed: the
  mint-authority model landed upstream in lez-programs#213 (no transfer
  authority, and none is needed); escrow custody works today with
  `Transfer` + `pda_seeds` (this program, single and batch, public and
  private payout).
- **Batch limit = 10 schedules/tx, enforced.** `MAX_BATCH_SCHEDULES = 10`
  is asserted by the handler; `batch_create_max_size_is_documented` bisects
  the batch size against the state machine and pins the runtime limit to it
  (11 fails with `MaxChainedCallsDepthExceeded`). The limit is `lee`'s
  `MAX_NUMBER_CHAINED_CALLS = 10` — one chained escrow-funding transfer per
  schedule — not transaction size: `max_block_size` (1 MiB default) minus
  the 200 B block header (sequencer `service.rs`) would admit thousands of
  schedules. Chained calls execute sequentially and each
  is validated against the state left by the previous one, so the handler
  hands call *i* a creator holding already debited *i* times.
- **Authorities must be program-owned accounts (e.g. token holdings).** A
  plain wallet account works as an authority exactly once: after its first
  signed transaction its nonce is non-zero, the spel-framework output filter
  drops it from the program output (it keeps default-owned accounts only while
  they are still in their default state), and LEZ v0.2.4 rejects the
  transaction with `DeclaredAccountMissingFromOutput`. Reproduced by
  `plain_wallet_authority_can_sign_only_once`. The runtime itself accepts an
  unchanged echo (it only rejects *modified* default-owned accounts without a
  claim), so this is a spel-framework filter to relax upstream; until then the
  creator holding is the default authority and nominated authorities are
  expected to be holdings.
- **Guest entry order = `Instruction` variant order.** The SPEL CLI encodes an
  instruction's enum discriminant from its *position in the IDL*, and the IDL
  lists instructions in guest-source order. A guest whose `claim` entry sits
  second while `Claim` is the third variant makes the CLI dispatch
  `CreateScheduleBatch` for `claim` (the transaction is silently dropped by
  the sequencer). `idl_instruction_order_matches_enum_discriminants` pins the
  order; `Timestamp`-typed fields are declared as `u64` so `spel inspect` can
  decode the state (the IDL has no alias table).
- **`spel-cli` cannot encode nested structs or `Vec<u128>` arguments** (only
  primitives, `Option`, `Vec<u8>`/`Vec<u32>`, program ids, strings). That is
  why `CreateSchedule` is flattened, `tranches` needed a small `spel-cli`
  patch (`Vec<primitive>` as a comma-separated list; 185 insertions, PR
  logos-co/spel#266, open), and
  `CreateScheduleBatch { params: Vec<ScheduleParams> }` remains SDK-only.
- **Zero-signer transactions share a hash.** An earlier, permissionless
  version of `Claim` carried no signature and no nonce, so repeated claims on
  the same accounts produced the same content-derived hash and the
  sequencer's `get_transaction` returned the earlier included claim for a
  repeat (see run 2 in the evidence file). Worse, anybody could force a
  public claim and defeat the beneficiary's later private one. Both are
  closed by design: claims are signed by the registered beneficiary holding,
  whose nonce makes every claim transaction unique.
- **Private claim = fresh private account, no clock account.** The claimant
  submits a privacy-preserving transaction whose private input initialises a
  new private account — `PrivateAuthorizedInit` under the claimant's own
  wallet key (`AccountIdentity::PrivateOwned`), which is what
  `tools/vesting-private-claim` and the testnet run use; the zkVM test
  `claim_to_fresh_private_account` uses `PrivateForeignInit` — and the chained
  Token Program transfer credits it. Observers see the schedule, the escrow,
  the signing beneficiary holding, the amount (in the schedule state and as
  the escrow delta) and the commitments / nullifiers / ciphertexts of the
  private message. What stays hidden is *which* account (key) holds the tokens
  and their later movements: spending the note produces an update nullifier
  that needs the nullifier secret key, and the creator never learns the
  private account. The opening balance is *not* hidden — it equals the public
  claim amount (1,000 here) and is inferable from the escrow delta. The
  account id must not be published either: LEZ's initialisation nullifier is
  a keyless hash of the account id (`Nullifier::for_account_initialization`
  = SHA256(`"/LEE/v0.3/Nullifier/Initialize/\0"` ‖ id)) and the note's
  commitment is recomputable from (id, owner = token program, balance = the
  public escrow delta, nonce = H(id), data), so anyone holding the id can pick
  the note out of the transaction and confirm its opening balance. The
  evidence therefore identifies the note by its init nullifier only.
  A privacy-preserving transaction is verified against the current pre-state
  of every declared public account, and `CLOCK_01` changes every block (~60 s
  on testnet 0.2), so a private claim that declared `CLOCK_01` would have to
  be proven and included within one block. `ClaimToPrivate { as_of }`
  therefore declares no clock account: the program computes the vested amount
  at `as_of` and pins `timestamp_validity_window = as_of..` on its output,
  which the runtime checks at inclusion against the block timestamp
  (`OutOfValidityWindow` before `as_of`), so the amount paid can never exceed
  what is vested at inclusion and the proof stays valid until included as
  long as its declared accounts are untouched. Two caveats: (1) the proof
  binds the pre-state (state and nonce) of the schedule, the escrow vault and
  the signing beneficiary holding, so any transaction that touches any of
  them before inclusion — a cancel, a milestone signal, another claim, or any
  token transfer *into* the beneficiary holding by anyone, even 1 unit —
  invalidates the pending proof and the claim must be re-proved (no state
  changes, nothing is lost); (2) a transaction whose validity window is not
  yet open (`as_of` in the future) is not queued — the sequencer leaves it
  out of the block and drops it from the mempool — so `as_of` must be ≤ the
  sequencer's clock (the tool refuses future values; default now − 60 s).
  `ClaimTo` with `CLOCK_01` remains the public-destination variant. Pinned by
  the zkVM test `private_claim_pins_validity_window_without_clock`, `private_claim_pays_vested_at_as_of_not_at_inclusion`, `private_claim_replay_is_rejected` and the
  handler tests `claim_to_private_*`; **ran on the public testnet** as run 4
  step R4-21 (`tools/vesting-private-claim`, sent from the beneficiary's
  wallet, `as_of` = 1787919126620 = now − 60 s; a few minutes — about six from
  tool start to inclusion in the recorded run, ~13 GB RAM): tx
  `3a2828ab85dcc503959447eb4a8734eaa4ab309fc6ef7f84869727c53b22c72b` (block `27362`), escrow
  17 → 0, the beneficiary's public holding unchanged, the new note identified
  by its init nullifier
  `c260c8e46b9b153039b66464cff5044315ff5e4c7c41f857febcafde1aa93293` (private action
  6 of 7) and decrypted to 1,000 by the wallet — recorded in
  `docs/vesting/testnet-private-claim.tsv` and checked by `verify-onchain.sh`
  (privacy-preserving tx, the three public ids present as a positive control,
  the init nullifier present among the private actions, validity window
  `from: Some(as_of), to: None`).

## Test results

- 17/17 core tests (vesting math for both kinds, cancellation freeze,
  parameter validation, event sequencing, IDL discriminant order, flat
  instruction round-trip, borsh round-trip)
- 41/41 handler tests (every instruction: happy path, each rejection with a
  deterministic message, post-state echo; `claim_to_private_*` ×4: pays vested
  at `as_of` and pins the window, rejects `as_of` before the cliff, requires
  the beneficiary signature, respects the cancellation freeze)
- 17/17 zkVM integration tests: escrow funded on creation; pre-cliff claim
  rejected; unsigned claim rejected; claim to a second public holding
  authorised by the beneficiary; pro-rata claim and remainder; cancel splits vested/unvested and
  the vested part stays claimable; unsigned cancel rejected; one-way
  non-cancelable; milestone signals unlock in order and a repeated signal is
  rejected without double-unlock; transferable position moves and the old
  beneficiary can no longer claim; non-transferable transfer rejected; batch
  maximum bisected; plain-wallet-authority constraint; claim to a fresh
  private account; private claim pins the validity window without a clock
  account (`private_claim_pins_validity_window_without_clock`); private claim
  pays exactly the amount vested at `as_of` when included later
  (`private_claim_pays_vested_at_as_of_not_at_inclusion`); private claim
  replay rejected (`private_claim_replay_is_rejected`).
- 75 tests in total (17 core · 41 handler · 17 zkVM).

## Measured cycle costs

Real `vesting` guest ELF through the RISC Zero executor
(`ExecutorImpl::run`, no proving), reproducing `lee`'s input encoding.

Measurement basis: the full vesting guest per instruction — SPEL account
decoding, PDA derivation and output encoding included; the chained Token
Program transfers execute in the token guest and are excluded — built from
the source at tag `testnet-0.2-evidence-v4` (testnet 0.2 ImageID
`952b031fecf9e357c76daca7c307e0debb4949d30d419b3c3a1db402a1fa46be`, program v4), LEZ
`v0.2.4`, RISC Zero 3.0.5. LEZ meters zkVM cycles, not compute units.

| Operation | user | paging | total | share of 32 MiCycle budget |
|-----------|------|--------|-------|-----------------------------|
| `CreateSchedule` | 266,163 | 52,141 | 524,288 | 1.6% |
| `Claim` | 376,495 | 58,893 | 524,288 | 1.6% |
| `ClaimTo` | 406,209 | 59,524 | 524,288 | 1.6% |
| `ClaimToPrivate` | 384,978 | 59,498 | 524,288 | 1.6% |
| `Cancel` | 436,715 | 60,194 | 524,288 | 1.6% |
| `MakeNonCancelable` | 235,849 | 51,162 | 524,288 | 1.6% |
| `SignalMilestone` | 252,423 | 51,214 | 524,288 | 1.6% |
| `TransferBeneficiary` | 259,285 | 51,497 | 524,288 | 1.6% |
| `CreateScheduleBatch` (n = 2) | 468,105 | 62,204 | 1,048,576 | 3.1% |
| `CreateScheduleBatch` (n = 10) | 2,059,076 | 160,261 | 2,359,296 | 7.0% |

Totals are segment-padded executor cycles: RISC Zero pads each segment to a
power of two and `total` sums the padded segments (524,288 = 2¹⁹;
2,359,296 = 2²¹ + 2¹⁸), while `user` and `paging` are unpadded. Every
single-schedule operation fits the public-execution budget with more than
an order of magnitude to spare; the largest batch uses 7% of it.

## RFP-017 requirement coverage

| Requirement | Where | Proof |
|---|---|---|
| F1 cliff+linear / fully linear / milestone | `ScheduleKind`, `vested_at` | core tests; `claim_pays_pro_rata_and_remainder_at_end`, `milestone_schedule_unlocks_per_signal_and_rejects_double_signal` |
| F2 beneficiary set at creation; claim to public or private account chosen at claim time; creator does not learn the private account | `ScheduleParams.beneficiary_holding_id`, `Claim`, `ClaimTo`, `ClaimToPrivate` | on chain (public: run 4 R4-6, R4-13, R4-19; private: `3a2828ab85dcc503959447eb4a8734eaa4ab309fc6ef7f84869727c53b22c72b`, block 27362, note identified by its init nullifier in `docs/vesting/testnet-private-claim.tsv` — the destination id is withheld, see *Platform findings*) + tests: `claim_pays_pro_rata_and_remainder_at_end`, `claim_to_a_second_public_holding_is_authorised_by_the_beneficiary`, `claim_to_fresh_private_account`, `private_claim_pins_validity_window_without_clock`, `private_claim_pays_vested_at_as_of_not_at_inclusion`, `private_claim_replay_is_rejected`, `unsigned_claim_is_rejected_so_nobody_can_force_a_public_claim`, handler tests `claim_to_private_*` |
| F3 cancelable by default (the program takes an explicit `cancelable` flag; the default is applied by the SDK/CLI/mini-app, M2–M3); one-way conversion; unvested returns, vested stays claimable | `ScheduleParams.cancelable`, `Cancel`, `MakeNonCancelable`, `cancelled_at` | `cancel_splits_vested_and_unvested`, `make_non_cancelable_blocks_cancellation_for_good`, handler tests |
| F4 transferability fixed at creation | `transferable`, `TransferBeneficiary` | `transferable_position_moves_to_new_beneficiary`, `non_transferable_position_rejects_transfer` |
| F5 batch creation with documented maximum | `CreateScheduleBatch` | `batch_create_max_size_is_documented` (max = 10) |
| F6 event per transition with documented schema | `VestingEvent`, `event_seq` | handler tests assert `last_event` per instruction (`Claim`, `ClaimTo` and `ClaimToPrivate` all emit `Claimed`) |
| R1 atomic claim / R2 atomic cancel | runtime atomicity; handlers panic before any post-state | rejected claims/cancels leave balances untouched (asserted in e2e) |
| R3 independent schedules | per-schedule account + escrow PDA | batch test funds 10 isolated escrows |
| R4 idempotent milestone signalling | index must equal next unsignalled | `signal_milestone_is_idempotent_per_index`, e2e double-signal |
| P1 one transaction per claim / P2 CU per operation | — | cycle table above |
| Soft: cancellation authority, milestone authority | `cancel_authority_id`, `milestone_authority_id` | `create_schedule_honours_nominated_authorities`, e2e with nominated holdings |
| U1–U7, Privacy 1–3 (SDK, CLI, mini-app, disclosures) | proposal milestones M2–M3 | — |

## Reproduce

```bash
export PATH="$HOME/.risc0/bin:$HOME/.cargo/bin:$PATH"

# Core + handler tests
RISC0_DEV_MODE=1 cargo test -p vesting_core -p vesting_program

# End-to-end through the zkVM state machine (prints MAX_BATCH_SCHEDULES)
RISC0_DEV_MODE=1 cargo test -p integration_tests --test vesting -- --nocapture

# Cycle benchmark (executor only, no proving)
RISC0_SKIP_BUILD_KERNELS=1 RISC0_DEV_MODE=1 cargo test \
  --manifest-path programs/benchmark/Cargo.toml \
  --test vesting_cycle_bench -- --ignored --nocapture

# Lints the upstream CI runs
RISC0_SKIP_BUILD=1 cargo clippy -p vesting_core -p vesting_program -p integration_tests --all-targets -- -D warnings
cargo clippy --manifest-path programs/vesting/methods/guest/Cargo.toml --all-targets -- -D warnings
```

## Proposal scope beyond this repository

SDK (Rust core module + C ABI), CLI, Basecamp mini-app with pre-claim
summary and privacy disclosure, live-sequencer CI, testnet 0.2 / 0.3 and
mainnet deployments — covered by the NODERS RFP-017 proposal this
implementation accompanies.

## License

The vesting program, its tests, tooling and documentation added in this
repository are dual-licensed under MIT ([`LICENSE`](LICENSE)) and
Apache-2.0 ([`LICENSE-APACHE`](LICENSE-APACHE)), as RFP-017 requires; the
upstream `lez-programs` code keeps its own MIT license.
