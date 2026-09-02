# Vesting program — spel runbook

How to deploy and use the **vesting** program end to end with `spel` on a LEZ sequencer
(public testnet or a local node). Program reference, event schema, cycle costs and RFP-017
traceability live in [`README-VESTING.md`](../../README-VESTING.md); the recorded public-testnet
run is in [`testnet-evidence.md`](./testnet-evidence.md).

**Verified against:** `lez-programs` main at tag `testnet-0.2-evidence-v4` (program v4, ImageID `952b031f…46be`, on top of upstream `6b53f3a`), LEZ **v0.2.4**,
`spel` CLI built from `logos-co/spel` `1ef0500f`, `wallet` from `logos-execution-zone` **v0.2.4**.

> **Golden rule (same as the AMM runbook):** every time you **recompile** the guest, its
> **ProgramId changes**, and **every escrow PDA derived from it changes too**. Re-derive escrow
> ids after a rebuild; never reuse old values. Existing schedules belong to the old program.

## 0. Prerequisites

- **Docker running** (guest builds cross-compile through it).
- **`spel`** from `logos-co/spel` at `1ef0500f` or later (`cargo install --path spel-cli --locked`)
  and **`wallet`** from `logos-execution-zone` **v0.2.4** (`cargo install --path lez/wallet --locked`
  inside the checkout). Both must target the same LEZ version as the sequencer;
  `wallet check-health` confirms it.
- **Wallet home** exported in every shell (deploy *and* spel must point at the same wallet):
  ```bash
  export LEE_WALLET_HOME_DIR="$HOME/.lee/wallet"
  ```
  with a `wallet_config.json` (v0.2.1 schema):
  ```json
  {
    "sequencers": [{ "sequencer_addr": "https://testnet.lez.logos.co/" }],
    "seq_poll_timeout": "3s",
    "seq_tx_poll_max_blocks": 40,
    "seq_poll_max_retries": 5,
    "seq_block_poll_max_amount": 100
  }
  ```
  (`seq_poll_timeout` is the *inter-poll delay*, not a total timeout — keep it short and raise
  the block count instead.)
- Testnet 0.2 charges no fees and program deployment needs no signer, so no faucet step is
  required.

### Argument formats

| Kind | Accepted forms |
|---|---|
| account id | base58 (e.g. `9qbX…`) or `0x`-prefixed 32-byte hex |
| program id | 8 comma-separated u32 limbs or the 64-char ImageID hex printed by `spel -- program-id` |
| timestamps | milliseconds since the Unix epoch (`u64`), as read from the LEZ clock account |
| `--tranches` | comma-separated `u128` list; **non-empty selects a milestone schedule** |
| `--cancel-authority-id` / `--milestone-authority-id` | `none` or an account id (`OPT<ACCOUNT_ID>`) |

## 1. Build & deploy

```bash
# token (any SPEL token program works; this repo's is used below) and vesting
cargo risczero build --manifest-path programs/token/methods/guest/Cargo.toml
cargo risczero build --manifest-path programs/vesting/methods/guest/Cargo.toml

TOKEN_BIN=programs/token/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/token.bin
VEST_BIN=programs/vesting/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/vesting.bin

wallet deploy-program "$TOKEN_BIN"
wallet deploy-program "$VEST_BIN"

spel -- program-id "$TOKEN_BIN"   # → TOKEN_PID (limbs + ImageID hex)
spel -- program-id "$VEST_BIN"    # → VEST_PID
make idl                          # artifacts/vesting-idl.json
```

## 2. Accounts

```bash
for l in vest-token-def vest-creator vest-beneficiary vest-beneficiary2 \
         vest-cancel-auth vest-milestone-auth vest-schedule-1; do
  wallet account new public --label "$l"
done
wallet account list
```

Roles:

| Label | Role | Must be |
|---|---|---|
| vest-token-def | token definition | fresh |
| vest-creator | creator's token holding (funds every escrow) | token holding |
| vest-beneficiary | beneficiary holding | token holding (`initialize-account`) |
| vest-cancel-auth / vest-milestone-auth | nominated authorities | **token holdings** — a plain wallet account can sign as an authority only once (see README-VESTING § Platform findings) |
| vest-schedule-N | schedule account (one per schedule) | fresh; signs its own creation |

## 3. Token fixture

```bash
TIDL=artifacts/token-idl.json
spel --idl $TIDL --program "$TOKEN_BIN" -- new-fungible-definition \
  --name "VEST-TEST" --total-supply 1000000 --mint-authority none \
  --definition-target-account <DEF> --holding-target-account <CREATOR>

for h in <BENEFICIARY> <BENEFICIARY2> <CANCEL_AUTH> <MILESTONE_AUTH>; do
  spel --idl $TIDL --program "$TOKEN_BIN" -- initialize-account \
    --definition-account <DEF> --account-to-initialize $h
done
```

## 4. Escrow PDA

```bash
cargo run -q -p vesting_program --example vesting_pdas -- <VEST_PID> <SCHEDULE_1>
# → <SCHEDULE_1>  escrow  <ESCROW_1>
```

## 5. Create a schedule

Cliff + linear (fully linear when `--cliff` equals `--start`), cancelable and transferable,
authorities nominated:

```bash
VIDL=artifacts/vesting-idl.json
spel --idl $VIDL --program "$VEST_BIN" -- create-schedule \
  --beneficiary-holding-id <BENEFICIARY> --total-amount 1000 \
  --start <T0_MS> --cliff <T0_MS+120000> --end <T0_MS+360000> --tranches "" \
  --cancelable true --transferable true \
  --cancel-authority-id <CANCEL_AUTH> --milestone-authority-id <MILESTONE_AUTH> \
  --token-program-id <TOKEN_PID> \
  --schedule-target <SCHEDULE_1> --creator-holding <CREATOR> --escrow <ESCROW_1>
```

Milestone schedule (three tranches summing to the total):

```bash
spel --idl $VIDL --program "$VEST_BIN" -- create-schedule \
  --beneficiary-holding-id <BENEFICIARY> --total-amount 1000 \
  --start 0 --cliff 0 --end 0 --tranches 300,700 \
  --cancelable true --transferable false \
  --cancel-authority-id <CANCEL_AUTH> --milestone-authority-id <MILESTONE_AUTH> \
  --token-program-id <TOKEN_PID> \
  --schedule-target <SCHEDULE_2> --creator-holding <CREATOR> --escrow <ESCROW_2>
```

Signers: the schedule target and the creator holding (both wallet-owned). Verify:

```bash
spel --idl $VIDL inspect <SCHEDULE_1> --type VestingSchedule
spel --idl $TIDL inspect <ESCROW_1>   --type TokenHolding      # balance == total
```

## 6. Claim (signed by the beneficiary)

```bash
CLOCK=4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWNU   # canonical LEZ clock account
spel --idl $VIDL --program "$VEST_BIN" -- claim \
  --schedule <SCHEDULE_1> --escrow <ESCROW_1> --beneficiary-holding <BENEFICIARY> --clock $CLOCK
```

The registered beneficiary holding signs (the wallet owns it), so nobody else can trigger a
claim and its nonce makes every claim transaction unique. Before the cliff the program rejects
with `Claim: nothing is claimable yet`; after it the holding grows by `claimable_at(now)` and
the schedule records `Claimed { amount }`.

To claim to a different public destination (another holding) use `claim-to`; the beneficiary
holding still signs and the clock account is declared as for `claim`:

```bash
spel --idl $VIDL --program "$VEST_BIN" -- claim-to \
  --schedule <SCHEDULE_1> --escrow <ESCROW_1> --beneficiary-holding <BENEFICIARY> \
  --destination <OTHER_HOLDING> --clock $CLOCK
```

(`claim-to` with `CLOCK_01` is the public-destination variant. A private destination needs
`ClaimToPrivate` in a privacy-preserving transaction, which the `spel` CLI does not build; use
the wallet-based tool in §7.)

## 7. Private claim (`ClaimToPrivate`)

`ClaimToPrivate { as_of }` pays what is vested at `as_of` to a fresh private account and pins
the output's `timestamp_validity_window` to `as_of..`; it declares no clock account, so the
privacy-preserving proof is not bound to `CLOCK_01` and can be included any time after `as_of`
— as long as the accounts it does declare stay untouched (see *Caveats* below).
The evidence tool `tools/vesting-private-claim` builds, proves (succinct, local CPU — a few
minutes; about six from start to inclusion in the recorded run on an M-series laptop, ~13 GB
RAM) and sends it from the wallet that owns the beneficiary holding:

```bash
export LEE_WALLET_HOME_DIR="$HOME/.lee/wallet"     # the beneficiary's wallet
cargo run -q -p vesting-private-claim -- \
  --vesting-bin "$VEST_BIN" --token-bin "$TOKEN_BIN" \
  --schedule <SCHEDULE> --escrow <ESCROW> --beneficiary-holding <BENEFICIARY> \
  [--as-of <MS>] [--label vest-private-1] [--expect-image-id <VEST_IMAGE_ID>] [--dry-run]
# → vesting/token ImageIDs, as_of, init nullifier, tx_hash, block, private_balance.
#   The destination id is printed only to the owner's terminal — do not publish it (see below).
```

As run on testnet 0.2 (run 4, step R4-21 in `testnet-evidence.md`: schedule 17, fully vested at
creation, 1,000 VEST-TEST; program v4):

```bash
cargo run -q -p vesting-private-claim -- \
  --vesting-bin "$VEST_BIN" --token-bin "$TOKEN_BIN" \
  --schedule DCupLBTdA6zzf7U2oPeL2SQpsaHvJihrXonwFsewx9yM \
  --escrow BZv7AChohMo4jGrtQue5Z6qyFu5bZKpfBBp3Xjmoz8zj \
  --beneficiary-holding 21RuJK97VnxdiDA7SYWckkPNyDosnurd8eEjfyiF6ASL \
  --expect-image-id 952b031fecf9e357c76daca7c307e0debb4949d30d419b3c3a1db402a1fa46be \
  --label vest-private-17
# → tx `3a2828ab85dcc503959447eb4a8734eaa4ab309fc6ef7f84869727c53b22c72b`, block `27362`
#   init nullifier: c260c8e46b9b153039b66464cff5044315ff5e4c7c41f857febcafde1aa93293
#   escrow 17 → 0, beneficiary holding unchanged, private_balance 1000 (decrypted by the wallet)
```

`--as-of` defaults to now − 60 s and the tool refuses a value in the future (a local clock
ahead of the sequencer must not push `as_of` into its future — see *Caveats*). `--dry-run`
prints the plan without creating an account, proving or sending. Proving goes through the external `r0vm` 3.0.5 binary (`~/.risc0/bin` on PATH,
`RISC0_DEV_MODE` unset), as the `wallet` CLI does; `--features prove` compiles the prover in
instead (needs the Xcode Metal toolchain on macOS). The tool first syncs the wallet to the
latest block — a wallet only ever used through `spel` starts at block 0, so the first run walks
the whole chain (about 7 minutes for 27k blocks); later runs are incremental. It persists the
wallet storage right after creating the destination and again after inclusion, and decrypts the
new note into the wallet so `private_balance` reads locally.

What is public: the schedule, the escrow, the signing beneficiary holding, the claimed amount
(schedule state and escrow delta) and the commitments / nullifiers / ciphertexts of the private
message (`wallet chain-info transaction --hash <TX>` shows them). What stays hidden: *which*
account (key) holds the tokens and their later movements (spending the note needs the nullifier
secret key). The opening balance is not hidden — it equals the public claim amount and is
inferable from the escrow delta. The destination id must not be published: LEZ's init nullifier
is a keyless hash of the account id and the note's commitment is recomputable from (id, token
program, balance = escrow delta, nonce = H(id), data), so anyone holding the id can pick the
note out of the transaction and confirm its opening balance. The evidence identifies the note
by its init nullifier (`docs/vesting/testnet-private-claim.tsv`, checked by
`scripts/vesting/verify-onchain.sh`).

Caveats:

- **The proof binds the pre-state of its declared accounts.** A privacy-preserving proof is
  valid against the current state and nonce of the schedule, the escrow vault *and* the signing
  beneficiary holding. Any transaction that touches any of them before inclusion — a cancel, a
  milestone signal, another claim, or any token transfer *into* the beneficiary holding by
  anyone, even 1 unit — invalidates the pending proof. Re-run the tool to re-prove the claim;
  no state changes and nothing is lost.
- **`as_of` must be ≤ the sequencer's clock.** A transaction whose validity window is not yet
  open is not queued: the sequencer leaves it out of the block and drops it from the mempool,
  with no client-visible error. The tool refuses a future `--as-of`; the default now − 60 s
  absorbs normal clock skew.

## 8. Milestones

```bash
spel --idl $VIDL --program "$VEST_BIN" -- signal-milestone --index 0 \
  --schedule <SCHEDULE_2> --milestone-authority <MILESTONE_AUTH>
# claim as in §6; signalling index 0 again is rejected ("milestone already signalled")
spel --idl $VIDL --program "$VEST_BIN" -- signal-milestone --index 1 \
  --schedule <SCHEDULE_2> --milestone-authority <MILESTONE_AUTH>
```

## 9. Transfer the position

```bash
spel --idl $VIDL --program "$VEST_BIN" -- transfer-beneficiary \
  --new-beneficiary-holding-id <BENEFICIARY2> \
  --schedule <SCHEDULE_1> --beneficiary-holding <BENEFICIARY>
```

Signed by the current beneficiary holding; afterwards only `<BENEFICIARY2>` can claim.

## 10. Cancel / make non-cancelable

```bash
spel --idl $VIDL --program "$VEST_BIN" -- cancel \
  --schedule <SCHEDULE_3> --escrow <ESCROW_3> --creator-holding <CREATOR> --clock $CLOCK \
  --cancel-authority <CANCEL_AUTH>
# unvested → creator holding; vested-but-unclaimed stays claimable by the beneficiary

spel --idl $VIDL --program "$VEST_BIN" -- make-non-cancelable \
  --schedule <SCHEDULE_4> --cancel-authority <CANCEL_AUTH>
# a later cancel is rejected ("schedule is non-cancelable")
```

## 11. Batch creation

`create-schedule-batch` takes `Vec<ScheduleParams>` (a nested struct), which the SPEL CLI cannot
encode; it is exercised by the integration test `batch_create_max_size_is_documented`
(maximum **10 schedules per transaction**) and is the SDK's entry point.

## Gotchas

- **Chained-call depth = 10.** More than 10 escrow transfers in one transaction fail with
  `MaxChainedCallsDepthExceeded`.
- **Authorities as plain wallet accounts sign once.** After their first signed transaction the
  spel-framework output filter drops them from the program output and the runtime rejects the
  transaction. Use token holdings as authorities.
- **Every declared account must be in the output** (LEZ v0.2.4). All vesting instructions echo
  their read-only accounts; if you write a new instruction, do the same.
- **Guest toolchain rustc is 1.88.** Pin `enum-ordinalize` to 4.3.2 in the guest lockfile
  (`cargo update -p enum-ordinalize --precise 4.3.2`) or the Docker build fails with
  "rustc 1.88.0-dev is not supported".
- **Two wallets.** `spel` reads the wallet through `LEE_WALLET_HOME_DIR`; older `spel` builds
  (v0.5.0 with LEZ v0.1.2) read `~/.nssa/wallet` and cannot sign for v0.2.x sequencers — build
  `spel` from `1ef0500f` or later.
