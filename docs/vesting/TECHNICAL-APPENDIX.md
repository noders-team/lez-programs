# Hourglass — Technical Appendix & Demo Evidence

Companion to the NODERS proposal for
[RFP-017 Privacy-Preserving Token Vesting](https://github.com/logos-co/rfp/blob/master/RFPs/RFP-017-token-vesting.md).
The proposal states what Hourglass is, what runs today and what the grant
funds; this appendix carries the mechanics and the proof. Everything here is
pinned to tag
[`testnet-0.2-evidence-v4`](https://github.com/noders-team/lez-programs/tree/testnet-0.2-evidence-v4)
(program v4, ImageID `952b031fecf9e357c76daca7c307e0debb4949d30d419b3c3a1db402a1fa46be`)
on top of upstream
[`logos-blockchain/lez-programs@6b53f3a`](https://github.com/logos-blockchain/lez-programs/commit/6b53f3a13f1c0846984f7f4d8e6082b15fa970b8),
LEZ v0.2.4, `spel-framework` `1ef0500f`, RISC Zero 3.0.5. Upstream `dev` was
last re-checked on 2026-09-02.

| Section | Contents |
|---|---|
| [A](#a-testnet-deployment-evidence) | Testnet deployment evidence |
| [B](#b-private-claim-execution-model) | Private claim execution model |
| [C](#c-escrow-pda-and-token-program-mechanics) | Escrow, PDA and Token Program mechanics |
| [D](#d-clock-and-as_of-design) | Clock and `as_of` design |
| [E](#e-platform-dependency-findings) | Platform dependency findings (LP-0012, LP-0013, LP-0015, RFP-001, SPEL, sequencer) |
| [F](#f-test-matrix) | Test matrix |
| [G](#g-cycle-measurements) | Cycle measurements |
| [H](#h-reproduction-commands) | Reproduction commands |
| [I](#i-known-platform-limitations) | Known platform limitations and the milestone that addresses each |

Related files in this repository:
[`README-VESTING.md`](../../README-VESTING.md) (instruction reference, state
layout, event schema), [`docs/vesting/README.md`](./README.md) (spel runbook),
[`docs/vesting/testnet-evidence.md`](./testnet-evidence.md) (every transaction
of every run), [`testnet-transactions.tsv`](./testnet-transactions.tsv),
[`testnet-balances.tsv`](./testnet-balances.tsv),
[`testnet-private-claim.tsv`](./testnet-private-claim.tsv),
[`scripts/vesting/verify-onchain.sh`](../../scripts/vesting/verify-onchain.sh).

---

## A. Testnet deployment evidence

Sequencer `https://testnet.lez.logos.co/`, explorer
`https://explorer.testnet.lez.logos.co/transaction/<hash>`.

| Item | Value |
|---|---|
| Program (v4, final) | ImageID `952b031fecf9e357c76daca7c307e0debb4949d30d419b3c3a1db402a1fa46be`; deploy tx [`5feaa17a…`](https://explorer.testnet.lez.logos.co/transaction/5feaa17ac617bf682cadd55e332fdbcdc8e4b727ade9fdc196f7da058528f775), block 27324 (2026-08-28 11:41 UTC); guest binary 343,332 bytes, built with `cargo risczero build` (`risczero/risc0-guest-builder:r0.1.88.0`) |
| Token program used by the run | ImageID `14b3bc6cc129d0359f9595ba0f60a13e4a8df08ffe8eb1a83251c47abecc67a8` (this repository's token program); definition VEST-TEST, supply 1,000,000 |
| Lifecycle run (run 4) | 21 steps: 15 included public transactions, 5 rejected by the program (not included, no receipt), 1 privacy-preserving claim; 8 of 9 instructions (`CreateScheduleBatch` is SDK-only, covered by the zkVM test) |
| Private claim | tx [`3a2828ab…`](https://explorer.testnet.lez.logos.co/transaction/3a2828ab85dcc503959447eb4a8734eaa4ab309fc6ef7f84869727c53b22c72b), block 27362: `ClaimToPrivate`, 1,000 tokens from the escrow to a fresh private account |
| Final balances | ben 5,439 · ben2 3,071 · creator 983,490 · escrow-16 1,000 (schedule 16: one-hour cliff, made non-cancelable, unclaimed at the end of the run) · escrow-17 0 · private note 1,000 (wallet-decrypted) |
| Re-verification | `scripts/vesting/verify-onchain.sh` against the live sequencer: 21 lifecycle steps (inclusion / non-inclusion), 8 final balances, 6 private-claim properties — ALL GOOD |

**What the run exercises.** Five schedules: cliff+linear, fully linear,
milestone, one-hour cliff, and an already-vested schedule for the private
claim. A pre-cliff claim and a second claim with nothing new vested are
rejected; a pro-rata claim pays 467/1000 after the cliff; the position is
transferred, the old beneficiary's claim is rejected and the new beneficiary
claims the remaining 533 after the end; milestone tranches 300 and 700 are
unlocked and claimed and the duplicate signal is rejected; a `ClaimTo` pays a
tranche to a different holding on the beneficiary's authority; a one-way
non-cancelable conversion is followed by a rejected cancel; a mid-schedule
cancel returns the 95 unvested tokens to the creator while the 905 vested stay
claimable; the fifth schedule is claimed privately. Schedule 13 was created
with separately nominated cancellation and milestone authorities. Step-by-step
tables with every transaction hash, and the superseded runs 2 and 3 against
program versions v2/v3, are in
[`testnet-evidence.md`](./testnet-evidence.md).

**What `verify-onchain.sh` checks.** Inclusion of every included transaction
(by hash, via `wallet chain-info transaction --hash`), non-inclusion of every
rejected one, the final public token balances against `testnet-balances.tsv`,
and for the private claim: that the transaction is privacy-preserving, that
its public data contains the three public accounts (schedule, escrow vault,
signing beneficiary holding — a positive control), that the note's
initialisation nullifier appears among the private actions, and that the
validity window is `from: Some(as_of), to: None`. The destination account id
is neither checked nor recorded: account ids never appear in the public data
of a privacy-preserving message by construction, so its absence would prove
nothing (see B.3 on why the id is withheld from the evidence).

**Reviewer access.** The repository is public from the day of filing. A fresh
schedule with a Logos-held beneficiary key can be provisioned on request so a
reviewer can run the private claim end to end with
`tools/vesting-private-claim`.

---

## B. Private claim execution model

### B.1 Authorisation and destinations

Every claim is signed by the registered beneficiary holding. `Claim` pays that
holding; `ClaimTo` pays any public destination the beneficiary names;
`ClaimToPrivate { as_of }` pays a fresh private account. Two consequences:
nobody can force a public claim and defeat the beneficiary's later private one
(an unsigned claim is rejected —
`unsigned_claim_is_rejected_so_nobody_can_force_a_public_claim`), and the
signature nonce makes every claim transaction unique (see E.7 for the
zero-signer hash collision this closes).

### B.2 Why the destination is a *fresh* private account

In a privacy-preserving transaction a private account can only be
*initialised* from a default pre-state: `PrivateAuthorizedInit` when the
claimant's wallet holds the key (the testnet transaction,
`AccountIdentity::PrivateOwned`) or `PrivateForeignInit` for a key it does not
hold (the zkVM test `claim_to_fresh_private_account`). Updating an existing
private account needs its nullifier key and a membership proof
(`PrivateAuthorizedUpdate`, LEZ v0.2.4 privacy circuit), which would bind the
proof to the destination's current private state on top of the three public
pre-states and shorten its useful life further. So the private destination is
*created* by the claim, not topped up — the same construction the LEZ wallet
uses for private transfers. Consolidating claimed notes into one private
account is a private self-transfer the beneficiary's wallet can make at any
time, invisible on chain and independent of the vesting program.

### B.3 What is public, what is hidden, what is withheld

- **Public**: the schedule, the escrow vault and the signing beneficiary
  holding (ids and state deltas); the claim amount (in `Claimed { amount }`
  and as the escrow delta); the commitments, nullifiers and ciphertexts of the
  private message (seven private actions in the recorded transaction); the
  validity window `as_of..`.
- **Hidden**: *which* account (key) holds the tokens and their later
  movements. Spending the note produces an update nullifier that needs the
  nullifier secret key; nothing in the public data links the note to a
  wallet. The creator never learns the private account.
- **Inferable, and not claimed as hidden**: the opening balance of the
  private note equals the public claim amount. A fresh private account that
  later unshields exactly N is linkable by amount. The mini-app's privacy
  disclosure (Privacy 2) states both.
- **Withheld from the evidence**: the destination account id. LEZ's
  initialisation nullifier is a keyless hash of the account id
  (`Nullifier::for_account_initialization` =
  SHA256(`"/LEE/v0.3/Nullifier/Initialize/\0"` ‖ id)) and the note's
  commitment is recomputable from (id, owner = token program, balance = the
  public escrow delta, nonce = H(id), data), so anyone holding the id can pick
  the note out of the transaction and confirm its opening balance. The
  evidence identifies the note by its init nullifier
  `c260c8e46b9b153039b66464cff5044315ff5e4c7c41f857febcafde1aa93293`
  (private action 6 of 7) only; `tools/vesting-private-claim` prints the id
  to the owner's terminal and nowhere else.

### B.4 The recorded transaction

Sent by `tools/vesting-private-claim` from the wallet that owns the
beneficiary holding `21RuJK97…` as a privacy-preserving transaction:
`as_of` = 1787919126620 (now − 60 s at tool start); destination initialised
in the same transaction under the beneficiary's own key; credited by the
chained Token Program transfer; proof produced locally (r0vm, succinct
receipt, ~13 GB RAM). Wall clock from tool start to inclusion: about six
minutes (12:13:03 → block 27362 at 12:19:55 UTC). Effect: escrow 17 → 0, the
beneficiary's public holding unchanged, the private balance reads 1,000 in the
wallet. Recorded in [`testnet-private-claim.tsv`](./testnet-private-claim.tsv)
and checked by `verify-onchain.sh`.

### B.5 Tests pinning the model

`claim_to_fresh_private_account`,
`private_claim_pins_validity_window_without_clock`,
`private_claim_pays_vested_at_as_of_not_at_inclusion`,
`private_claim_replay_is_rejected` (zkVM state machine); `claim_to_private_*`
×4 (handler: pays vested at `as_of` and pins the window, rejects `as_of`
before the cliff, requires the beneficiary signature, respects the
cancellation freeze).

### B.6 Path to the SDK (M2)

The evidence tool's wallet path moves behind the SDK's C ABI as an async
prove / submit / re-prove state machine with proving-progress reporting for
the mini-app, the `as_of` policy of D.3, and pre-flight validation that the
destination is a private account (Privacy 3). Tightenings left for M1:
`ClaimToPrivate` rejects a non-default destination in the guest and accepts an
optional `valid_until` upper bound.

---

## C. Escrow, PDA and Token Program mechanics

### C.1 Account model

One schedule account (program-owned, claimed at creation) and one escrow
vault PDA per schedule, derived as `sha256("VESTING_ESCROW" || schedule_id)`.
Schedule state: creator, beneficiary and escrow ids; token program id; total
and claimed amounts; `kind` (`CliffLinear { start, cliff, end }` or
`Milestone { tranches }`); `milestones_signalled`; `cancelable`,
`transferable`; `cancelled_at`; cancellation and milestone authorities;
`event_seq`, `last_event`. Full layout in
[`README-VESTING.md`](../../README-VESTING.md#schedule-state).

Instructions: `CreateSchedule` (flattened, CLI-callable),
`CreateScheduleBatch` (≤ 10, SDK), `Claim`, `ClaimTo`, `Cancel`,
`MakeNonCancelable`, `SignalMilestone { index }`, `TransferBeneficiary`,
`ClaimToPrivate { as_of }`.

### C.2 Custody without a transfer-authority primitive

The vault is funded at creation and paid out at claim and cancel by chained
Token Program `Transfer` calls authorised with `ChainedCall::pda_seeds` — the
pattern the upstream AMM uses for its own vaults. The RFP lists LP-0013 as an
open custody blocker; LP-0013 was in fact awarded and landed upstream as a
*mint*-authority model
([`logos-blockchain/lez-programs#213`](https://github.com/logos-blockchain/lez-programs/pull/213):
`mint_authority`, `SetAuthority`) rather than by merging the prize PR. It
adds no transfer/approve primitive, and escrow needs none. Exercised on
testnet for single and batch funding and for public and private payout.

### C.3 Token program binding

The schedule stores the token program id the creator supplied; the token
definition is that of the escrow vault holding. SDK, CLI, mini-app and indexer
pin the token program to the network's canonical Token Program id and flag any
schedule naming another one. **Known defect in the deployed program**: the
beneficiary (and new-beneficiary) holding is not validated against the
schedule's token definition at creation and transfer, so a wrong-token holding
would leave a non-cancelable schedule unclaimable. Closed in M1.

### C.4 Batch creation and the vault-per-schedule decision

The RFP asks for the maximum bounded by the transaction size limit. On LEZ
v0.2.4 that limit is `max_block_size` (1 MiB default) minus the 200 B block
header (sequencer `service.rs`), which would admit thousands of schedules; the
binding limit is the runtime's `MAX_NUMBER_CHAINED_CALLS = 10`, because every
schedule is funded by its own chained transfer into its own vault. The
documented maximum is therefore **10 schedules per transaction**
(`MAX_BATCH_SCHEDULES = 10`, asserted by the handler and pinned to the runtime
limit by `batch_create_max_size_is_documented`, which bisects the batch size
against the state machine — 11 fails with `MaxChainedCallsDepthExceeded`).
The SDK splits a larger list into transactions of 10. Chained calls execute
sequentially and each is validated against the state left by the previous
one, so the handler hands call *i* a creator holding already debited *i*
times.

A per-batch shared vault would lift the cap into the thousands and was
rejected deliberately: every private claim's proof binds the current
pre-state of the vault it draws from, so a vault shared with other schedules
would invalidate a pending private claim whenever any sibling schedule is
claimed or cancelled. One vault per schedule keeps every claim, including
private ones, isolated in its own escrow.

### C.5 Atomicity (R1, R2, R3)

A claim is one transaction: the schedule update and the chained transfer are
one call chain that the runtime applies only if every call succeeds
(LP-0015, delivered in the core runtime). The same holds for cancel.
Handlers panic before producing any post-state, so a rejected claim or cancel
leaves balances untouched — asserted on chain by the five rejected steps of
run 4. Schedules are independent accounts with independent vaults; the batch
test funds ten isolated vaults.

### C.6 Vesting math

Cliff+linear uses floor pro-rata `q·elapsed + (r·elapsed)/duration` with
`q = total / duration` and `r = total mod duration`: both products stay below
2¹²⁸ for any `u128` total and any millisecond duration below 2⁶⁴, so no
widening is needed and overflow is impossible; `vested(end) == total`
exactly. The lump released at the cliff is the amount accrued since `start`
(for "1-year cliff, 3-year linear release" that is 25 % at the cliff — the
RFP appendix's "4 years total" example); M1 adds an optional `cliff_amount`
so a creator can set the cliff lump explicitly with the remainder linear to
`end`. Fully linear is `cliff == start`. Milestone tranches are unlocked by
the milestone authority by explicit index, in order: a repeated index fails
deterministically and never double-unlocks (R4); a skipped index is rejected
too, because tranches are ordered deliverables and in-order signalling keeps
"next unlock" well-defined. `Cancel` freezes accrual at `cancelled_at`; after
a cancel, vested-at-`as_of` for a private claim is clamped to `cancelled_at`.

---

## D. Clock and `as_of` design

### D.1 The liveness problem

A privacy-preserving transaction is verified against the *current* pre-state
of every public account it declares (`validated_state_diff`, LEZ v0.2.4).
`CLOCK_01` changes every block (~60 s on testnet 0.2), so a proof that read
the clock account would be includable only in the next block, while real
proving takes minutes.

### D.2 The design

`ClaimToPrivate { as_of }` declares no clock account. The program computes the
vested amount at `as_of` and pins `ProgramOutput::timestamp_validity_window`
with `as_of` as its lower bound — a bound the runtime checks at inclusion
against the block timestamp (`OutOfValidityWindow`). The transaction can
therefore never pay more than was vested at inclusion time. The window has no
upper bound of its own: the proof stays valid until included or until the
schedule, the vault or the signing holding changes. Public `Claim` and
`ClaimTo` keep reading `CLOCK_01`, because the sequencer executes them at
inclusion.

### D.3 `as_of` policy

`as_of` is chosen by the claimant and can only hurt the claimant: a value in
the past under-pays (the remainder stays claimable); a value in the future is
not includable, and the sequencer does not queue it — it leaves the
transaction out of the block and drops it from the mempool. The SDK and the
evidence tool set `as_of = now − 60 s` and refuse future values. "Claimable
now" and "next unlock" in the mini-app are computed locally from the public
schedule state.

### D.4 Pending-proof invalidation and the griefing vector

A pending proof is invalidated by any transaction that touches the schedule,
the vault or the beneficiary holding — a cancel, a signal, another claim, or
any transfer *into* that holding. Nothing changes on chain (R1); the SDK
detects the changed pre-state after the `getTransaction` timeout, reports the
cause and re-proves (U7).

This is also an adversarial vector: anyone — the creator included — can
invalidate a pending private claim by sending one token unit to the
beneficiary holding, at the price of one public transaction per attempt
against several minutes of proving on the claimant's side. The program cannot
close it (the holding must sign, so the runtime binds its pre-state). The
clients mitigate it: the SDK re-proves automatically and names the interfering
transaction; the mini-app recommends registering a dedicated holding as
beneficiary that receives nothing else; every such transfer is public and
attributable. The privacy-properties document delivered in M4 records this
next to amount-linkability.

### D.5 Tests

`private_claim_pins_validity_window_without_clock` (runtime rejects inclusion
before `as_of`), `private_claim_pays_vested_at_as_of_not_at_inclusion`
(exactly the amount vested at `as_of` when included later),
`private_claim_replay_is_rejected` (replay → nonce mismatch).

---

## E. Platform dependency findings

Each finding is reproduced by a test or a command in this repository.

### E.1 LP-0013 — token authorities (RFP: "open"; actually closed, awarded)

Specifies a *mint*-authority model, landed by the LEZ team in
[`lez-programs#213`](https://github.com/logos-blockchain/lez-programs/pull/213)
(`mint_authority`, `SetAuthority`) rather than by merging the prize PR. No
transfer/approve primitive was added and escrow needs none: custody is
`Transfer` authorised via `ChainedCall::pda_seeds` (C.2).

### E.2 LP-0015 — general cross-program calls (delivered)

Delivered by the LEZ team in the core runtime. Hourglass uses chained calls
for every escrow movement; the runtime applies a call chain only if every call
succeeds, which is what makes claim and cancel atomic (C.5). The 10-call
limit per transaction bounds batch creation (C.4).

### E.3 LP-0012 — events (RFP: "closed"; no API in v0.2.4)

The prize implementation was not merged, and LEZ v0.2.4 (testnet 0.2) has no
event API — `ProgramOutput` carries no events. On 2026-08-27 the LEZ team
merged its own event system into `dev`:
[`logos-execution-zone#705`](https://github.com/logos-blockchain/logos-execution-zone/pull/705)
`feat!: events` (`ProgramOutput::with_events`),
[`#707`](https://github.com/logos-blockchain/logos-execution-zone/pull/707)
8-byte selectors, indexer `getEvents` / `subscribeToEvents`; unreleased.

Today Hourglass ships an **event seam**: `event_seq` + `last_event:
VestingEvent` in the schedule account, read per transaction by indexers.
`VestingEvent` (`Created`, `Claimed`, `Cancelled`, `MadeNonCancelable`,
`MilestoneSignalled`, `BeneficiaryTransferred`) maps 1:1 onto
`ProgramEvent { selector, data }` and is wired to the runtime mechanism in
the testnet 0.3 milestone. An indexer that finds a gap in `event_seq`
re-reads the schedule account, whose fields (`claimed`, `cancelled_at`,
`milestones_signalled`, `beneficiary`) are the source of truth, so derived
state is always recoverable. The runtime does not commit events for
privacy-preserving transactions (#705), so a private claim is recorded only in
the schedule account — F6 for private claims is bounded by the runtime, not
by this program. Schema in
[`README-VESTING.md`](../../README-VESTING.md#event-schema-f6).

### E.4 RFP-001 — admin authority (soft dependency)

Awarded and shipped as external crates (`spel-admin-authority` v0.1.2) that
depend on the SPEL extension mechanism proposed in
[`spel#257`](https://github.com/logos-co/spel/pull/257) (open); nothing is in
upstream SPEL or `lez-programs`, and the authority library proposed in
[`lez-programs#125`](https://github.com/logos-blockchain/lez-programs/pull/125)
was closed without merge. The fee switch (M1) sits behind a thin trait with an
inline PDA-held admin account — the pattern the upstream AMM uses for its own
admin — and a migration seam to the library if it lands.

### E.5 LEZ runtime rules observed

- **Declared accounts must appear in the output.** LEZ v0.2.4 rejects a
  transaction unless every declared account appears in the accumulated output
  of the call chain (`DeclaredAccountMissingFromOutput`); every Hourglass
  instruction echoes its read-only accounts.
- **Plain wallets as authorities sign once.** After its first signed
  transaction a plain wallet account's nonce is non-zero; the spel-framework
  output filter drops it from the program output (it keeps default-owned
  accounts only while still in their default state) and the runtime rejects
  the transaction with `DeclaredAccountMissingFromOutput`. The runtime itself
  would accept an unchanged echo (it only rejects *modified* default-owned
  accounts without a claim), so this is a framework filter. Reproduced by
  `plain_wallet_authority_can_sign_only_once`. Today nominated authorities
  must be program-owned accounts — token holdings, which is also what
  multisigs and DAOs hold. The upstream idiom is
  `AccountPostState::new_claimed_if_default`
  ([`spel#262`](https://github.com/logos-co/spel/pull/262), in our pinned
  revision); adopted for nominated authorities in M1 so a plain wallet can be
  an authority too.
- **10 chained calls per transaction** (`MAX_NUMBER_CHAINED_CALLS`), see C.4.
- **Rejected transactions are silent.** The RPC rejects only stateless
  failures (size, signature, sequencer-only program) synchronously; execution
  failures are logged server-side and the transaction is skipped — not
  included, no receipt (`getTransaction` returns null). The CLI/SDK therefore
  compute the RFP's actionable errors ("nothing claimable — next unlock at …")
  locally before submitting, and on a pre-flight-clean claim that is
  nevertheless skipped (raced by a cancel or transfer) the SDK times out on
  `getTransaction`, re-reads the schedule and reports the cause from state.
  The only way to debug a platform rejection is the local state-machine test.

### E.6 SPEL CLI and IDL findings

- **Guest entry order = `Instruction` variant order.** The SPEL CLI encodes an
  instruction's enum discriminant from its *position in the IDL*, and the IDL
  lists instructions in guest-source order. The first testnet deployment (v1)
  had `claim` second in the guest while `Claim` is the third variant, so the
  CLI dispatched `CreateScheduleBatch` for every `claim` and the sequencer
  dropped them silently. Pinned by
  `idl_instruction_order_matches_enum_discriminants`. `Timestamp`-typed
  fields are declared as `u64` so `spel inspect` can decode the state (the IDL
  has no alias table).
- **`spel-cli` cannot encode nested structs or `Vec<u128>` arguments** (only
  primitives, `Option`, `Vec<u8>`/`Vec<u32>`, program ids, strings).
  `CreateSchedule` is therefore flattened (CLI-callable without any patch);
  milestone tranches on the CLI use our `spel-cli` patch (`Vec<primitive>` as
  a comma-separated list, 185 insertions), open upstream as
  [logos-co/spel#266](https://github.com/logos-co/spel/pull/266);
  `CreateScheduleBatch { params: Vec<ScheduleParams> }` stays SDK-only. The
  CLI covers 7 of 9 instructions; the private claim goes through
  `tools/vesting-private-claim`.
- **Borsh migration ahead.** LEZ `dev` has moved instruction data to borsh end
  to end
  ([`logos-execution-zone@5a393d2`](https://github.com/logos-blockchain/logos-execution-zone/commit/5a393d2),
  2026-08-23), so the CLI serializer (and the patch) is re-based for testnet
  0.3 in M6.

### E.7 Zero-signer transactions share a hash (closed by design)

An earlier, permissionless version of `Claim` carried no signature and no
nonce, so repeated claims on the same accounts produced the same
content-derived hash and the sequencer's `getTransaction` returned the earlier
included claim for a repeat (run 2 in the evidence file). Worse, anybody could
force a public claim and defeat the beneficiary's later private one. Both are
closed: claims are signed by the registered beneficiary holding, whose nonce
makes every claim transaction unique.

---

## F. Test matrix

75 tests: 17 core · 41 handler · 17 zkVM state-machine integration
(`programs/integration_tests`). CI on `main`: unit, handler, zkVM tests,
clippy (host and guest), fmt, IDL check.

| Suite | Count | Covers |
|---|---|---|
| Core | 17 | vesting math for both kinds, cancellation freeze, parameter validation, event sequencing, IDL discriminant order, flat instruction round-trip, borsh round-trip |
| Handler | 41 | every instruction: happy path, each rejection with a deterministic message, post-state echo; `claim_to_private_*` ×4 |
| zkVM integration | 17 | escrow funded on creation; pre-cliff claim rejected; unsigned claim rejected; claim to a second public holding authorised by the beneficiary; pro-rata claim and remainder; cancel splits vested/unvested and the vested part stays claimable; unsigned cancel rejected; one-way non-cancelable; milestone signals unlock in order and a repeated signal is rejected without double-unlock; transferable position moves and the old beneficiary can no longer claim; non-transferable transfer rejected; batch maximum bisected; plain-wallet-authority constraint; claim to a fresh private account; private claim pins the validity window without a clock account; private claim pays exactly the amount vested at `as_of` when included later; private claim replay rejected |

Hard requirement → test mapping:

| Requirement | Where | Proof |
|---|---|---|
| F1 cliff+linear / fully linear / milestone | `ScheduleKind`, `vested_at` | core tests; `claim_pays_pro_rata_and_remainder_at_end`, `milestone_schedule_unlocks_per_signal_and_rejects_double_signal` |
| F2 beneficiary set at creation; claim to public or private account; creator does not learn the private account | `Claim`, `ClaimTo`, `ClaimToPrivate` | on chain (public: run 4 R4-6, R4-13, R4-19; private: `3a2828ab…`, block 27362) + `claim_pays_pro_rata_and_remainder_at_end`, `claim_to_a_second_public_holding_is_authorised_by_the_beneficiary`, `claim_to_fresh_private_account`, `private_claim_*` ×3, `unsigned_claim_is_rejected_so_nobody_can_force_a_public_claim`, handler `claim_to_private_*` |
| F3 cancelable by default (program takes an explicit flag; default applied by SDK/CLI/mini-app); one-way conversion; unvested returns, vested stays claimable | `Cancel`, `MakeNonCancelable`, `cancelled_at` | `cancel_splits_vested_and_unvested`, `make_non_cancelable_blocks_cancellation_for_good`, handler tests |
| F4 transferability fixed at creation | `transferable`, `TransferBeneficiary` | `transferable_position_moves_to_new_beneficiary`, `non_transferable_position_rejects_transfer` |
| F5 batch creation with documented maximum | `CreateScheduleBatch` | `batch_create_max_size_is_documented` (max = 10) |
| F6 event per transition with documented schema | `VestingEvent`, `event_seq` | handler tests assert `last_event` per instruction |
| R1 / R2 atomic claim and cancel | runtime atomicity; handlers panic before any post-state | rejected claims/cancels leave balances untouched (asserted on chain, run 4) |
| R3 independent schedules | per-schedule account + escrow PDA | batch test funds 10 isolated escrows |
| R4 idempotent milestone signalling | index must equal next unsignalled | `signal_milestone_is_idempotent_per_index`, e2e double-signal |
| P1 / P2 | — | every claim is one transaction; cycle table (G) |
| Soft: cancellation and milestone authorities | `cancel_authority_id`, `milestone_authority_id` | `create_schedule_honours_nominated_authorities`, run 4 schedule 13 with nominated holdings |
| U1–U7, Privacy 1–3 | proposal milestones M2–M3 | — |

M1 adds: tests for `total == 0`, a wrong clock account id, cancel before the
cliff and cancel of a milestone schedule (unvested = unsignalled tranches);
property tests on the vesting math; explicit R1/R2 tests with a failing
chained transfer; sequencer e2e in CI (standalone mode).

---

## G. Cycle measurements

Real `vesting` guest ELF through the RISC Zero executor (`ExecutorImpl::run`,
no proving), reproducing `lee`'s input encoding. Measurement basis: the full
vesting guest per instruction — SPEL account decoding, PDA derivation and
output encoding included; the chained Token Program transfers execute in the
token guest and are excluded (added to the table in M1). Source at tag
`testnet-0.2-evidence-v4`, LEZ v0.2.4, RISC Zero 3.0.5. LEZ meters zkVM
cycles, not compute units; the public-execution budget is
`MAX_NUM_CYCLES_PUBLIC_EXECUTION` = 32 Mi (2²⁵ = 33,554,432) cycles.

| Operation | user | paging | total (padded) | share of budget |
|---|---|---|---|---|
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
power of two and `total` sums the padded segments (524,288 = 2¹⁹; 2,359,296 =
2²¹ + 2¹⁸), while `user` and `paging` are unpadded. Every single-schedule
operation fits the budget with more than an order of magnitude to spare; the
largest batch uses 7 %. Re-measured on every LEZ release (M4, M6, M7).

Private-claim proving on the client: a few minutes of local r0vm proving
(~13 GB RAM); about six minutes from tool start to inclusion in the recorded
run.

---

## H. Reproduction commands

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

# Re-verify the recorded testnet run against the live sequencer
# (21 lifecycle steps, 8 balances, 6 private-claim properties)
scripts/vesting/verify-onchain.sh

# Rebuild the guest and compare the ImageID (expects 952b031f…46be at the tag;
# reproducible only through the Docker builder risczero/risc0-guest-builder:r0.1.88.0,
# see the runbook's gotchas for the enum-ordinalize pin)
cargo risczero build --manifest-path programs/vesting/methods/guest/Cargo.toml
```

Deploying and driving the program with `spel` (accounts, token fixture,
escrow PDA derivation, each instruction, the private claim with
`tools/vesting-private-claim`) is the step-by-step runbook in
[`docs/vesting/README.md`](./README.md).

---

## I. Known platform limitations

| # | Limitation | Consequence today | Addressed |
|---|---|---|---|
| 1 | No runtime event API in LEZ v0.2.4; events not committed for privacy-preserving transactions (E.3) | Event seam in schedule state; private claims visible only through schedule state | M6 wires the seam to runtime events on testnet 0.3; private-tx events remain a runtime property |
| 2 | Private claim cannot read the clock account (D.1) | `as_of` + validity-window design; claimant chooses `as_of` | Design shipped; `valid_until` upper bound in M1 |
| 3 | Pending private proof invalidated by any touch of schedule, vault or beneficiary holding, including a 1-unit transfer by a third party (D.4) | Re-prove (minutes); nothing lost | SDK auto re-prove + attribution (M2); dedicated-holding guidance in mini-app (M3); documented in M4 privacy-properties note |
| 4 | Proving latency: a few minutes of local proving, about six minutes to inclusion in the recorded run, ~13 GB RAM on the client (B.4) | Private claim is a "start and wait" action | Progress reporting in SDK/mini-app (M2/M3); tracks LEZ/RISC Zero releases (Post-Delivery) |
| 5 | Plain wallet accounts can act as authority only once (E.5) | Nominated authorities must be holdings | `new_claimed_if_default` in M1 |
| 6 | 10 chained calls per transaction (C.4) | Batch max 10 schedules | SDK splits lists; documented maximum |
| 7 | `spel-cli` cannot encode nested structs / `Vec<u128>` (E.6) | 7 of 9 instructions on the stock CLI; tranches need patch spel#266; batch SDK-only | M2 CLI; borsh re-base in M6 |
| 8 | Rejected transactions produce no receipt (E.5) | Errors must be computed client-side | Local pre-flight + state re-read in SDK (M2) |
| 9 | Beneficiary holding not validated against token definition (C.3) | Known defect in the deployed program | M1 |
| 10 | Opening balance of the private note equals the public claim amount (B.3) | Amount-linkability | Stated in the privacy disclosure (M3) and the M4 privacy-properties note |
