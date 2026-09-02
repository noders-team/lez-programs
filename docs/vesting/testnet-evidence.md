# Vesting program — testnet 0.2 evidence

Sequencer: `https://testnet.lez.logos.co/` · Explorer: `https://explorer.testnet.lez.logos.co/transaction/<hash>`
Wallet: `lez/wallet` from `logos-execution-zone` v0.2.4 (`wallet check-health` ✅ against testnet), `spel` CLI built from `logos-co/spel` @ `1ef0500f`.

## Build

| Program | ImageID | Binary | Built from |
|---|---|---|---|
| token | `14b3bc6cc129d0359f9595ba0f60a13e4a8df08ffe8eb1a83251c47abecc67a8` | `programs/token/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/token.bin` | `cargo risczero build` (risczero/risc0-guest-builder:r0.1.88.0), lez-programs at tag `testnet-0.2-evidence` |
| vesting v1 (superseded) | `f9b3333aa98a3deb443e0cb6f9be7165884e500d61b9f40549818722a7360f02` | — | intermediate build, not tagged: guest entry order mismatch (see note), same builder |
| vesting v2 (superseded) | `1b4d3cfe3bd93b474d892775bb39871516c23844dbada6e38e5db2a2da5343b2` | — | intermediate build, not tagged: zero-signer `Claim` (see run 2), same builder |
| vesting v3 (superseded) | `e22f2b7bd843d7d5d1c178e64f9112138c2dc312759d6124cc5c6efdfdda5741` | — | same builder, lez-programs at tag `testnet-0.2-evidence`; deploy tx [`20419916…`](https://explorer.testnet.lez.logos.co/transaction/20419916d07f58be201f9ccdc13b70009e9556550b8e2f91dfa8877ada570868), block 26952; run 3 (still deployed, no `ClaimToPrivate`) |
| **vesting v4 (final)** | `952b031fecf9e357c76daca7c307e0debb4949d30d419b3c3a1db402a1fa46be` | `programs/vesting/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/vesting.bin` (343,332 bytes) | same builder, lez-programs at tag `testnet-0.2-evidence-v4`; adds `ClaimToPrivate { as_of }` (discriminant 8, last); deploy tx [`5feaa17a…`](https://explorer.testnet.lez.logos.co/transaction/5feaa17ac617bf682cadd55e332fdbcdc8e4b727ade9fdc196f7da058528f775), block 27324 (2026-08-28 11:41 UTC); run 4 |

## Accounts (wallet labels)

| Label | Account id |
|---|---|
| vest-token-def | `3Smnnce1JymQagXJCxehWiBVN4jKc13K2qDDPttnmkq9` |
| vest-creator | `FzmatEJbAggHEzh7KoUeDjngg9jWVAboNrZKyDRmDG71` |
| vest-beneficiary | `21RuJK97VnxdiDA7SYWckkPNyDosnurd8eEjfyiF6ASL` |
| vest-beneficiary2 | `6C7DWacAjppTMdFNSAS8cv5VEexugyd9W56o2B8p1Xx3` |
| vest-cancel-auth | `5jX9bx5MS3GdcBH7GdnGoWg9m913eS7pZxc69Lbd21e3` |
| vest-milestone-auth | `AXFWdhmkHjvuEJwfvivFFMcAHQvWr9WPXwYG937PTiPD` |
| vest-schedule-1 | `G8psqsYkhmsTzifFrL73AJuczh9iUERnsb51CJNsELw2` |
| vest-schedule-2 | `2G36ffq5KSRJEGdrZrefJCg25zYAfeXGCYy7LXjTUKmf` |
| vest-schedule-3 | `C8s9VrWdGk6n8wsCS3efvvqiJkAHyJNA9yFzj1VjzVDE` |
| vest-schedule-4 | `A358ihp1RPqqaJw1oZZ3LZuQ9AUWzWGPhNSqUwT5SDiM` |
| vest-batch-a | `2iqtxaqwgD9WApGo8Pfsfwx2PLrY8MVAxfoEY9TyAeuW` |
| vest-batch-b | `FK31n6YGxtWu4rFnekR1AW39gjHrKphaDZVCExAKcsg9` |

## Setup transactions (steps 1–10)

Step numbers restart per run: setup 1–10 here, R2-1…R2-19 in run 2, R3-1…R3-19 in run 3,
R4-1…R4-21 in run 4.

| # | Step | Tx | Block | Result |
|---|---|---|---|---|
| 1 | deploy token program (ImageID `14b3bc6c…67a8`) | [`43fab3ef2a83e7037c389c8c33a4126ab0f066e87e3611dac3e3bea04dae83e8`](https://explorer.testnet.lez.logos.co/transaction/43fab3ef2a83e7037c389c8c33a4126ab0f066e87e3611dac3e3bea04dae83e8) | 25517 | included |
| 2 | `new-fungible-definition` VEST-TEST, supply 1,000,000 → vest-creator | [`51a3c08bacb61c178b093745aa0702f02208c685d10b3b0994095d49527efb73`](https://explorer.testnet.lez.logos.co/transaction/51a3c08bacb61c178b093745aa0702f02208c685d10b3b0994095d49527efb73) | | confirmed |
| 3 | `initialize-account` vest-beneficiary | [`8698ca8babda7146d1e86e885eb38b81aadc8c7ee1e752a287035e1e7a28022e`](https://explorer.testnet.lez.logos.co/transaction/8698ca8babda7146d1e86e885eb38b81aadc8c7ee1e752a287035e1e7a28022e) | | confirmed |
| 4 | `initialize-account` vest-beneficiary2 | [`2c6cf1f9a2b0415b20a50a88fec8b140b27bd188b1590e6a81bf7f8df8e14e4a`](https://explorer.testnet.lez.logos.co/transaction/2c6cf1f9a2b0415b20a50a88fec8b140b27bd188b1590e6a81bf7f8df8e14e4a) | | confirmed |
| 5 | `initialize-account` vest-cancel-auth | [`d9c11e9e78457b4a1f54263e9bef0bc9c6301b3d3f7ea4d73cc832444909c882`](https://explorer.testnet.lez.logos.co/transaction/d9c11e9e78457b4a1f54263e9bef0bc9c6301b3d3f7ea4d73cc832444909c882) | | confirmed |
| 6 | `initialize-account` vest-milestone-auth | [`f5cd4fd73de3fab051c07cd4f6a61d819a03fb26fec24a256e4329b177f7d15f`](https://explorer.testnet.lez.logos.co/transaction/f5cd4fd73de3fab051c07cd4f6a61d819a03fb26fec24a256e4329b177f7d15f) | | confirmed |
| 7 | deploy vesting program v1 (ImageID `f9b3333a…0f02`, superseded) | [`88bf032c007c35b5c003936ad1d2f4569e6c82339cd27a94f413663d872b436d`](https://explorer.testnet.lez.logos.co/transaction/88bf032c007c35b5c003936ad1d2f4569e6c82339cd27a94f413663d872b436d) | 26812 | included |
| 8 | deploy vesting program v2 (ImageID `1b4d3cfe…43b2`, superseded) | [`5c22516f3c3da2f9724be689e3c0279ec8b838f9640ced2470ebe19ace8ce558`](https://explorer.testnet.lez.logos.co/transaction/5c22516f3c3da2f9724be689e3c0279ec8b838f9640ced2470ebe19ace8ce558) | 26876 | included |
| 9 | deploy vesting program v3 (ImageID `e22f2b7b…5741`, superseded) | [`20419916d07f58be201f9ccdc13b70009e9556550b8e2f91dfa8877ada570868`](https://explorer.testnet.lez.logos.co/transaction/20419916d07f58be201f9ccdc13b70009e9556550b8e2f91dfa8877ada570868) | 26952 | included |
| 10 | deploy vesting program v4 **final** (ImageID `952b031f…46be`, adds `ClaimToPrivate`) | [`5feaa17ac617bf682cadd55e332fdbcdc8e4b727ade9fdc196f7da058528f775`](https://explorer.testnet.lez.logos.co/transaction/5feaa17ac617bf682cadd55e332fdbcdc8e4b727ade9fdc196f7da058528f775) | 27324 | included |

### Note on the first deployment (v1, `f9b3333a…`)

Run 1 against v1 created four schedules, transferred a position, signalled two
milestones, made a schedule non-cancelable and cancelled a fully-vested one —
all included — but **no `claim` was ever included**. Cause: the SPEL CLI
encodes an instruction's enum discriminant from its position in the IDL, which
follows guest-source order; `claim` was declared second in the guest while
`Claim` is the third `Instruction` variant, so the CLI sent
`CreateScheduleBatch` with no arguments and the sequencer dropped the
transaction (rejected transactions are not included; there is no client-visible
error). Fixed (guest order = enum order, regression test
`idl_instruction_order_matches_enum_discriminants`), rebuilt and redeployed as
v2. The v1 transactions remain on chain as evidence of the other instructions;
run 2 below is the complete lifecycle against v2.

## Escrow PDAs (derived from the vesting ImageID)

Run 4 (v4 final `952b031f…`; the escrows are the `escrow-13..17` rows of `testnet-balances.tsv`):

| Schedule | Escrow |
|---|---|
| schedule 13 `3UNdwPKNV8DxjDWe4n1MSwTqEQPqV1PPWgxvCPD6xddu` | `7ZnxdvcesYwLWngYaw9EwZwygNU1QKPzNdvcRM4bdqcv` |
| schedule 14 `BjSkNNmaQLnJBMQzUNxrZ27qvDunkWeBvjAk3tCZ1jRE` | `91RH21kwuj4HnefhbPRusBfoZcLXpCttZXpvhMNe1ets` |
| schedule 15 `FssyedaE9xHB1hPwgmbQpVKDu973W9UggumBCA2XxCtp` | `ACiWrpZ1n7HuSY7DtP2tFQu5hrt3TAEcaPC4bHmX18X9` |
| schedule 16 `2dJwsxhQW2tadDKHMLMK9FUrMm2eN8REpP5nkJQRkgDk` | `DzxumpD4uz9WzfzU31tPgqw3MV2LGrJY6a76yy62sJ82` |
| schedule 17 `DCupLBTdA6zzf7U2oPeL2SQpsaHvJihrXonwFsewx9yM` | `BZv7AChohMo4jGrtQue5Z6qyFu5bZKpfBBp3Xjmoz8zj` |

Run 3 (v3 `e22f2b7b…`, superseded; these were the `escrow-9..12` rows of `testnet-balances.tsv`
until run 4 replaced them):

| Schedule | Escrow |
|---|---|
| schedule 9 `2NdVZFnL26pYxJKV7ujREDWH9Br6fsBa6aqeWB8NjSn4` | `Bp6TErJjMnsgb4grteqWiQVzNqZrN1kbvziCPjkfhZ5C` |
| schedule 10 `7SYcvG6qQSSLL9xjVdNeAbKssKEvxLmxZY1Nv1vix2on` | `FfJmQLrfdHsXTMp88iWZPd6URrZMJ9ie3RHbm4sF1m8n` |
| schedule 11 `DfZ6TcdayX3CaqgQj4HowQEAKFHkr1P8cnSGmNviyXtp` | `3S45DT9FZjMojUpAoWveKYGQRXzMkC7dLMvTuta6ci6U` |
| schedule 12 `7Vz6NMSZLkYR5wJSccotD6iT87vP1BgTYWJg9iZdZvib` | `Dgj4RmngNWohNTY1UNT7mSZjXXSL9YxdfXQ9tfMYEqcD` |

Run 2 (v2 `1b4d3cfe…`):

| Schedule | Escrow |
|---|---|
| vest-schedule-5 `7DCoojRVPeQVR7njz92zgUMLcmkSWnbEhMP3P6KAQqWK` | `FqHDRbS3BXG3UnFRzZNxme5n8EKsmqiiDxjqERjskXuP` |
| vest-schedule-6 `CoEc67ui11uEcGhnJKN34wqFbD9AgYgnUMsA8jHswdVY` | `G2N4BU2jVvEkJU1ReSqM8wd1asyAJKjZg8LnpMeeA7Ac` |
| vest-schedule-7 `GV4pU5cJJ4Nkx7nXLqxJBJhyKc6FFecRmxpUxgNSmXDN` | `8mpbLkppskXGj2bpja8opavZ377oHSCU4kgcexxhg6cc` |
| vest-schedule-8 `J21bLNuEzJkReEmxQQo1wGYEQtbDajqeGdh2B5NUaGBZ` | `GRwuQAHC9o7SweCxXtptQ23o5NwJyURCcAsokQdEw3JG` |

Run 1 (v1 `f9b3333a…`):

| Schedule | Escrow |
|---|---|
| vest-schedule-1 `G8psqsYkhmsTzifFrL73AJuczh9iUERnsb51CJNsELw2` | `DZPMjjWjXLCBhWpsfjkYGLb6zoABaRwJxLGfQt4KjVkR` |
| vest-schedule-2 `2G36ffq5KSRJEGdrZrefJCg25zYAfeXGCYy7LXjTUKmf` | `2WqiGUVfRjtgkqKmfPNEBCqxKf4jfUoCRbfuJpgMeuD9` |
| vest-schedule-3 `C8s9VrWdGk6n8wsCS3efvvqiJkAHyJNA9yFzj1VjzVDE` | `3FrFH55V6MmwHKYiQX9ZomBKnHWTqe4S7tzrA9TvY5YK` |
| vest-schedule-4 `A358ihp1RPqqaJw1oZZ3LZuQ9AUWzWGPhNSqUwT5SDiM` | `GuVoMpFrMR3LWowhUL4Ecu8dxezyEx39nqMQZzhL8HQw` |

## Run 2 — full lifecycle against v2 (`1b4d3cfe…`), 2026-08-28 04:12–04:34 UTC — superseded

Run 2 exercised a permissionless (zero-signer) `Claim`. It surfaced the shared-hash problem
described under *Platform findings* in `README-VESTING.md` and the privacy hole a forced public
claim would open; the final program signs claims with the beneficiary holding and adds `ClaimTo`.
Run 3 below is the evidence for the final program; run 2 stays as the record of the finding.

Token program `14b3bc6c…67a8`, token `VEST-TEST`, creator holding funded with 1,000,000.
Timeline: T0 = 1787890369000 ms (04:12:49 UTC). Schedule 5: cliff T0+120 s, end T0+900 s;
schedule 7: fully linear over 1200 s from 04:17:03; schedule 8: 1-hour cliff.

| # | Step | Tx | Result |
|---|---|---|---|
| R2-1 | create schedule 5: cliff+linear, start T0, cliff T0+120s, end T0+900s, cancelable, transferable, nominated authorities | [`74290a8d…`](https://explorer.testnet.lez.logos.co/transaction/74290a8d567a9cd97b02e6b069be254ad72cfc6da63719c4249b95d5dc37fe3f) | included |
| R2-2 | claim schedule 5 before the cliff (expect rejection: nothing claimable yet) | [`00f6f7ae…`](https://explorer.testnet.lez.logos.co/transaction/00f6f7ae35107af0096a0163af5e61b6e05977dc64f3f65847049c796127765a) | NOT INCLUDED (unexpected) |
| R2-3 | create schedule 6: milestone tranches 300,700 | [`dbd2ec9e…`](https://explorer.testnet.lez.logos.co/transaction/dbd2ec9e739d38377a92ca61771d117f5344159edd59cb210e48250336495277) | included |
| R2-4 | create schedule 7: fully linear, start T7, end T7+1200s, cancelable | [`1a6847c5…`](https://explorer.testnet.lez.logos.co/transaction/1a6847c56dbb0bad31b3cd27dde96da28b21f25ae954175f64210571240ce1f6) | included |
| R2-5 | create schedule 8: cliff+linear over 1h, cancelable (to be made non-cancelable) | [`74fc928a…`](https://explorer.testnet.lez.logos.co/transaction/74fc928a65ce3b7273ec390bc26eed8d436a136b3dcac9b959ca629da070b789) | included |
| R2-6 | claim schedule 5 after the cliff (pro-rata, permissionless, zero signers) | [`00f6f7ae…`](https://explorer.testnet.lez.logos.co/transaction/00f6f7ae35107af0096a0163af5e61b6e05977dc64f3f65847049c796127765a) | included |
| R2-7 | transfer schedule 5 position to beneficiary2 (signed by current beneficiary) | [`3c439f03…`](https://explorer.testnet.lez.logos.co/transaction/3c439f0387ef8afe934ff3e14d54bc980385a133420dcf7297d0dd9c3db29acb) | included |
| R2-8 | claim schedule 5 to the OLD beneficiary (expect rejection: holding does not match the schedule) | [`00f6f7ae…`](https://explorer.testnet.lez.logos.co/transaction/00f6f7ae35107af0096a0163af5e61b6e05977dc64f3f65847049c796127765a) | included |
| R2-9 | signal milestone 0 on schedule 6 | [`f96df0e8…`](https://explorer.testnet.lez.logos.co/transaction/f96df0e8f3d038c6bcb5c15c0be1c747f993a30b550c3848492065eb7e34167a) | included |
| R2-10 | claim schedule 6 (tranche 0 = 300) | [`d67c4e9a…`](https://explorer.testnet.lez.logos.co/transaction/d67c4e9a32c3b2b6b5d731df27af1b43736b8025040efeeca9ae59bb15ec564a) | included |
| R2-11 | signal milestone 0 AGAIN on schedule 6 (expect rejection: already signalled) | [`3d60da0e…`](https://explorer.testnet.lez.logos.co/transaction/3d60da0e165c06f9436ba47ea932dc6ff5c54a44a952f4ea7eca74c4f9d72e98) | rejected — not included (expected) |
| R2-12 | signal milestone 1 on schedule 6 | [`414d3502…`](https://explorer.testnet.lez.logos.co/transaction/414d3502d977d30ec17f3216a53a9c22770729cfab51c056b8f0bdc8f81c1a7d) | included |
| R2-13 | claim schedule 6 (tranche 1 = 700) | [`d67c4e9a…`](https://explorer.testnet.lez.logos.co/transaction/d67c4e9a32c3b2b6b5d731df27af1b43736b8025040efeeca9ae59bb15ec564a) | included |
| R2-14 | make schedule 8 non-cancelable | [`4c9f83d6…`](https://explorer.testnet.lez.logos.co/transaction/4c9f83d6a3e08ff09ca4b2b0abec0f8beced93bad474de29781a2559023536c2) | included |
| R2-15 | cancel schedule 8 (expect rejection: non-cancelable) | [`bdae594a…`](https://explorer.testnet.lez.logos.co/transaction/bdae594a456e518905a2860006d56ccd499255c5d0b7631f5208bdf51a47eba8) | rejected — not included (expected) |
| R2-16 | cancel schedule 7 mid-way (unvested returns to creator, vested stays claimable) | [`b80b4fb8…`](https://explorer.testnet.lez.logos.co/transaction/b80b4fb8b8669bc7cdfd945316bd33cfcd3fa8af4142252c69fe483c11f4fceb) | included |
| R2-17 | claim schedule 7 after cancellation (vested-but-unclaimed part) | [`4a0fce4a…`](https://explorer.testnet.lez.logos.co/transaction/4a0fce4a28b4153d37e85d499a80e6ef86545188e3c0d9fe50e9be10de76c235) | included |
| R2-18 | claim schedule 7 again (expect rejection: nothing claimable) | [`4a0fce4a…`](https://explorer.testnet.lez.logos.co/transaction/4a0fce4a28b4153d37e85d499a80e6ef86545188e3c0d9fe50e9be10de76c235) | included |
| R2-19 | claim schedule 5 to beneficiary2 after the end (remainder) | [`2416a95a…`](https://explorer.testnet.lez.logos.co/transaction/2416a95a0285fd00c8359354b0f15b3678a8fa400f66d227ba9dfdcfc4afd798) | included |

"rejected — not included" means the sequencer executed the transaction, the program panicked
with its deterministic error, and the transaction was left out of the block (LEZ does not
include failing transactions and exposes no client-visible error). Balances after each of
those steps are unchanged (see the log below). Two repeats of an already-included
permissionless claim (`claim … to the OLD beneficiary`, `claim schedule 7 again`) and the
pre-cliff claim carry the **same hash** as the included claim (no signer, no nonce, identical
message), so the sequencer reports the earlier transaction for them; the balance checkpoints
prove the repeats moved nothing. See *Platform findings* in `README-VESTING.md`.

### Balance checkpoints (VEST-TEST)

| After | beneficiary | beneficiary2 | creator | escrow 5 | escrow 6 | escrow 7 | escrow 8 |
|---|---|---|---|---|---|---|---|
| create 5 | 0 | 0 | 995,000 | 1,000 | — | — | — |
| pre-cliff claim (rejected) | 0 | 0 | 995,000 | 1,000 | — | — | — |
| create 6, 7, 8 | 0 | 0 | 992,000 | 1,000 | 1,000 | 1,000 | 1,000 |
| claim 5 after cliff (pro-rata) | **415** | 0 | 992,000 | 585 | 1,000 | 1,000 | 1,000 |
| transfer 5 → beneficiary2; old beneficiary claim (no effect) | 415 | 0 | 992,000 | 585 | 1,000 | 1,000 | 1,000 |
| signal 0; claim 6 | 715 | 0 | 992,000 | 585 | 700 | 1,000 | 1,000 |
| signal 0 again (rejected); signal 1; claim 6 | 1,415 | 0 | 992,000 | 585 | 0 | 1,000 | 1,000 |
| make 8 non-cancelable; cancel 8 (rejected) | 1,415 | 0 | 992,000 | 585 | 0 | 1,000 | 1,000 |
| cancel 7 mid-way | 1,415 | 0 | **992,298** | 585 | 0 | 702 | 1,000 |
| claim 7 (vested part) | **2,117** | 0 | 992,298 | 585 | 0 | 0 | 1,000 |
| claim 5 by beneficiary2 after end | 2,117 | **585** | 992,298 | 0 | 0 | 0 | 1,000 |

Reconciliation: beneficiary 2,117 = 415 (schedule 5 pro-rata) + 300 + 700 (schedule 6) + 702
(schedule 7 vested at cancellation); beneficiary2 585 = schedule 5 remainder; creator 992,298 =
996,000 − 4 × 1,000 + 298 (schedule 7 unvested); escrow 8 untouched (non-cancelable, cliff not
reached).

### `scripts/vesting/verify-onchain.sh` output (run 2)

Produced with an earlier revision of the script that had a `dup` expectation for repeated
zero-signer claims. The current script accepts only `ok`/`reject`, and the TSV inputs now hold
run 3; this run-2 output is kept as a record only.

```
== transactions (docs/vesting/testnet-transactions.tsv)
✅ ok       74290a8d567a…  create schedule 5: cliff+linear, start T0, cliff T0+120s, end T0+900s, cancelable, transferable, nominated authorities
✅ dup      00f6f7ae3510…  claim schedule 5 before the cliff (expect rejection: nothing claimable yet)
✅ ok       dbd2ec9e739d…  create schedule 6: milestone tranches 300,700
✅ ok       1a6847c56dbb…  create schedule 7: fully linear, start T7, end T7+1200s, cancelable
✅ ok       74fc928a65ce…  create schedule 8: cliff+linear over 1h, cancelable (to be made non-cancelable)
✅ ok       00f6f7ae3510…  claim schedule 5 after the cliff (pro-rata, permissionless, zero signers)
✅ ok       3c439f0387ef…  transfer schedule 5 position to beneficiary2 (signed by current beneficiary)
✅ dup      00f6f7ae3510…  claim schedule 5 to the OLD beneficiary (expect rejection: holding does not match the schedule)
✅ ok       f96df0e8f3d0…  signal milestone 0 on schedule 6
✅ ok       d67c4e9a32c3…  claim schedule 6 (tranche 0 = 300)
✅ reject   3d60da0e165c…  signal milestone 0 AGAIN on schedule 6 (expect rejection: already signalled)
✅ ok       414d3502d977…  signal milestone 1 on schedule 6
✅ ok       d67c4e9a32c3…  claim schedule 6 (tranche 1 = 700)
✅ ok       4c9f83d6a3e0…  make schedule 8 non-cancelable
✅ reject   bdae594a456e…  cancel schedule 8 (expect rejection: non-cancelable)
✅ ok       b80b4fb8b866…  cancel schedule 7 mid-way (unvested returns to creator, vested stays claimable)
✅ ok       4a0fce4a28b4…  claim schedule 7 after cancellation (vested-but-unclaimed part)
✅ dup      4a0fce4a28b4…  claim schedule 7 again (expect rejection: nothing claimable)
✅ ok       2416a95a0285…  claim schedule 5 to beneficiary2 after the end (remainder)
== final balances
✅ balance vest-beneficiary 2117
✅ balance vest-beneficiary2 585
✅ balance vest-creator     992298
✅ balance escrow-5         0
✅ balance escrow-6         0
✅ balance escrow-7         0
✅ balance escrow-8         1000
== checked 19 transactions
ALL GOOD
```

Inputs: `docs/vesting/testnet-transactions.tsv`, `docs/vesting/testnet-balances.tsv`.

Reproduce an ImageID from source: `cargo risczero build --manifest-path programs/vesting/methods/guest/Cargo.toml`
(Docker, image `risczero/risc0-guest-builder:r0.1.88.0`) prints it, and
`spel -- program-id programs/vesting/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/vesting.bin`
prints it again from the binary. Run 3 (v3, `e22f2b7b…`) was built from the git tag
`testnet-0.2-evidence` of this repository and run 4 (v4, the final program, `952b031f…`) from
the tag `testnet-0.2-evidence-v4`; v1 and v2 were intermediate, untagged builds.

## Run 3 — program v3 (`e22f2b7b…`), 2026-08-28 05:28–05:57 UTC — superseded by run 4

Run 3 is the complete public lifecycle against v3, which differs from the final v4 only by the
absence of `ClaimToPrivate`. It remains valid history: v3 stays deployed, its transactions stay on
chain and the balances below are the starting point of run 4. Run 4 repeats the same 19 steps
against v4 and adds the private claim; the TSV inputs of `verify-onchain.sh` now hold run 4.

Token program `14b3bc6c…67a8`, token `VEST-TEST`; balances carry over from run 2 (beneficiary
2,117; beneficiary2 585; creator 992,298). T0 = 05:28:37 UTC. Schedule 9: cliff T0+120 s, end
T0+900 s; schedule 11: fully linear over 1,200 s from 05:33:20; schedule 12: 1-hour cliff.
Every claim is signed by the registered beneficiary holding, so every transaction — including
the rejected ones — has a unique hash. Schedule accounts 9–12 and their escrow PDAs are listed
under *Escrow PDAs* above (escrows = `escrow-9..12` in `testnet-balances.tsv`).

| # | Step | Tx | Result |
|---|---|---|---|
| R3-1 | create schedule 9: cliff+linear, start T0, cliff T0+120s, end T0+900s, cancelable, transferable, nominated authorities | [`ca6459b4…`](https://explorer.testnet.lez.logos.co/transaction/ca6459b4e208f0eb4b7138dee3f6563cc257005484bbd4f378d1075b8ea19f47) | included |
| R3-2 | claim schedule 9 before the cliff (expect rejection: nothing claimable yet) | [`0466b4cb…`](https://explorer.testnet.lez.logos.co/transaction/0466b4cbd44b5095bc18b6aad78ab4414c2d43b8ff6c5decb58f7478b9508135) | rejected — not included (expected) |
| R3-3 | create schedule 10: milestone tranches 300,700 | [`9967f73f…`](https://explorer.testnet.lez.logos.co/transaction/9967f73f8c44d620be50c2d9066920e8ab06874bf9548767d80b02ff17bf6ece) | included |
| R3-4 | create schedule 11: fully linear, start T7, end T7+1200s, cancelable | [`84889c3c…`](https://explorer.testnet.lez.logos.co/transaction/84889c3cd762428f112c5d9c03a04cb6e2c02c0916d89335ecf22901828322af) | included |
| R3-5 | create schedule 12: cliff+linear over 1h, cancelable (to be made non-cancelable) | [`da260fe6…`](https://explorer.testnet.lez.logos.co/transaction/da260fe6e1b9153de85d3d280645587a8d14424eb778c6e4f7e7265e93a71c01) | included |
| R3-6 | claim schedule 9 after the cliff (pro-rata, signed by the beneficiary) | [`380200ea…`](https://explorer.testnet.lez.logos.co/transaction/380200ea7c1731e9873f0e8ed3b04ca5f65d51e171e54d3f4cfe2fc95e2c6e3a) | included |
| R3-7 | transfer schedule 9 position to beneficiary2 (signed by current beneficiary) | [`ac5d8c17…`](https://explorer.testnet.lez.logos.co/transaction/ac5d8c17ad7fd17797a681e32f4ae99401518d731e440b1c9ebd65d8f90a4622) | included |
| R3-8 | claim schedule 9 signed by the OLD beneficiary (expect rejection: holding does not match the schedule) | [`aa7203a9…`](https://explorer.testnet.lez.logos.co/transaction/aa7203a94a24000dfaf2cf1e117c6f9ece9f2f088d6b2afd76f1822aa832d906) | rejected — not included (expected) |
| R3-9 | signal milestone 0 on schedule 10 | [`bd3b35b1…`](https://explorer.testnet.lez.logos.co/transaction/bd3b35b122bb7c45124bde2e41cf756afacbd34feb7077a7a75a420a3dfa8a80) | included |
| R3-10 | claim schedule 10 (tranche 0 = 300) | [`ecfb9ab9…`](https://explorer.testnet.lez.logos.co/transaction/ecfb9ab95532200e81f75b4625f6721830e5a261a7d253dab6e9951a808729e9) | included |
| R3-11 | signal milestone 0 AGAIN on schedule 10 (expect rejection: already signalled) | [`459c8d2e…`](https://explorer.testnet.lez.logos.co/transaction/459c8d2eefc7e5be3365a5183211062055b0089e153f7b10e7564b46bea96561) | rejected — not included (expected) |
| R3-12 | signal milestone 1 on schedule 10 | [`171a2a20…`](https://explorer.testnet.lez.logos.co/transaction/171a2a20c97378c4196f760a78f7d06081481b6b08e5290daeaeb0beb330876b) | included |
| R3-13 | claim-to: schedule 10 tranche 1 (700) paid to beneficiary2's holding, authorised by the beneficiary | [`1d444098…`](https://explorer.testnet.lez.logos.co/transaction/1d444098147095fe93ba918e55cada68cf4429f0de29f601db28a4aa6bcde6d1) | included |
| R3-14 | make schedule 12 non-cancelable | [`5ad39a78…`](https://explorer.testnet.lez.logos.co/transaction/5ad39a781f997167473638645444c7d1ba62cedd68f142e9f72b9d8342a8760d) | included |
| R3-15 | cancel schedule 12 (expect rejection: non-cancelable) | [`cf1f0ed3…`](https://explorer.testnet.lez.logos.co/transaction/cf1f0ed3adda0f1c2ee98b2e6bb35aff7db026643aa4f8df9f69a8066e729a6d) | rejected — not included (expected) |
| R3-16 | cancel schedule 11 mid-way (unvested returns to creator, vested stays claimable) | [`73bdb552…`](https://explorer.testnet.lez.logos.co/transaction/73bdb5525488cf9aa7ef6f1aea7a3f0005d5d63b28382dc74dd2ca5e8a410c63) | included |
| R3-17 | claim schedule 11 after cancellation (vested-but-unclaimed part) | [`ae95f5f8…`](https://explorer.testnet.lez.logos.co/transaction/ae95f5f883befcbc5b140c96dda5540e8ba877a3a73b8464f5de99b58754d22d) | included |
| R3-18 | claim schedule 11 again (expect rejection: nothing claimable) | [`9302e7df…`](https://explorer.testnet.lez.logos.co/transaction/9302e7df70424162ca43a686ac1c139536a3383bbaebdc3785948675e2dd9e3b) | rejected — not included (expected) |
| R3-19 | claim schedule 9 signed by beneficiary2 after the end (remainder) | [`476078b3…`](https://explorer.testnet.lez.logos.co/transaction/476078b33c1bdcb16d58e542fb70c36638fc0f221efac097a6c7bd3a5f3da78d) | included |

"rejected — not included" means the sequencer executed the transaction, the program panicked
with its deterministic error, and the transaction was left out of the block (LEZ does not
include failing transactions and exposes no client-visible error); the balance checkpoints
after each of those steps are unchanged.

### Balance checkpoints (VEST-TEST)

| After | beneficiary | beneficiary2 | creator | escrow 9 | escrow 10 | escrow 11 | escrow 12 |
|---|---|---|---|---|---|---|---|
| create 9 | 2,117 | 585 | 991,298 | 1,000 | — | — | — |
| pre-cliff claim (rejected) | 2,117 | 585 | 991,298 | 1,000 | — | — | — |
| create 10, 11, 12 | 2,117 | 585 | 988,298 | 1,000 | 1,000 | 1,000 | 1,000 |
| claim 9 after cliff (pro-rata, beneficiary-signed) | **2,564** | 585 | 988,298 | 553 | 1,000 | 1,000 | 1,000 |
| transfer 9 → beneficiary2; old beneficiary's claim rejected | 2,564 | 585 | 988,298 | 553 | 1,000 | 1,000 | 1,000 |
| signal 0; claim 10 | 2,864 | 585 | 988,298 | 553 | 700 | 1,000 | 1,000 |
| signal 0 again (rejected); signal 1; `claim-to` 10 → beneficiary2 | 2,864 | **1,285** | 988,298 | 553 | 0 | 1,000 | 1,000 |
| make 12 non-cancelable; cancel 12 (rejected) | 2,864 | 1,285 | 988,298 | 553 | 0 | 1,000 | 1,000 |
| cancel 11 mid-way | 2,864 | 1,285 | **988,395** | 553 | 0 | 903 | 1,000 |
| claim 11 (vested part); claim again (rejected) | **3,767** | 1,285 | 988,395 | 553 | 0 | 0 | 1,000 |
| claim 9 by beneficiary2 after end | 3,767 | **1,838** | 988,395 | 0 | 0 | 0 | 1,000 |

Reconciliation: beneficiary 3,767 = 2,117 + 447 (schedule 9 pro-rata) + 300 (schedule 10
tranche 0) + 903 (schedule 11 vested at cancellation); beneficiary2 1,838 = 585 + 700
(`claim-to` of tranche 1) + 553 (schedule 9 remainder); creator 988,395 = 992,298 − 4 × 1,000 +
97 (schedule 11 unvested); escrow 12 untouched (non-cancelable, cliff not reached).

### `scripts/vesting/verify-onchain.sh` output (run 3)

Produced while the TSV inputs held run 3 (script revision without the private-claim section);
kept as a record.

```
== transactions (docs/vesting/testnet-transactions.tsv)
✅ ok       ca6459b4e208…  create schedule 9: cliff+linear, start T0, cliff T0+120s, end T0+900s, cancelable, transferable, nominated authorities
✅ reject   0466b4cbd44b…  claim schedule 9 before the cliff (expect rejection: nothing claimable yet)
✅ ok       9967f73f8c44…  create schedule 10: milestone tranches 300,700
✅ ok       84889c3cd762…  create schedule 11: fully linear, start T7, end T7+1200s, cancelable
✅ ok       da260fe6e1b9…  create schedule 12: cliff+linear over 1h, cancelable (to be made non-cancelable)
✅ ok       380200ea7c17…  claim schedule 9 after the cliff (pro-rata, signed by the beneficiary)
✅ ok       ac5d8c17ad7f…  transfer schedule 9 position to beneficiary2 (signed by current beneficiary)
✅ reject   aa7203a94a24…  claim schedule 9 signed by the OLD beneficiary (expect rejection: holding does not match the schedule)
✅ ok       bd3b35b122bb…  signal milestone 0 on schedule 10
✅ ok       ecfb9ab95532…  claim schedule 10 (tranche 0 = 300)
✅ reject   459c8d2eefc7…  signal milestone 0 AGAIN on schedule 10 (expect rejection: already signalled)
✅ ok       171a2a20c973…  signal milestone 1 on schedule 10
✅ ok       1d4440981470…  claim-to: schedule 10 tranche 1 (700) paid to beneficiary2's holding, authorised by the beneficiary
✅ ok       5ad39a781f99…  make schedule 12 non-cancelable
✅ reject   cf1f0ed3adda…  cancel schedule 12 (expect rejection: non-cancelable)
✅ ok       73bdb5525488…  cancel schedule 11 mid-way (unvested returns to creator, vested stays claimable)
✅ ok       ae95f5f883be…  claim schedule 11 after cancellation (vested-but-unclaimed part)
✅ reject   9302e7df7042…  claim schedule 11 again (expect rejection: nothing claimable)
✅ ok       476078b33c1b…  claim schedule 9 signed by beneficiary2 after the end (remainder)
== final balances
✅ balance vest-beneficiary 3767
✅ balance vest-beneficiary2 1838
✅ balance vest-creator     988395
✅ balance escrow-9         0
✅ balance escrow-10        0
✅ balance escrow-11        0
✅ balance escrow-12        1000
== checked 19 transactions
ALL GOOD
```

Inputs at the time: `docs/vesting/testnet-transactions.tsv`, `docs/vesting/testnet-balances.tsv`
(run 3; both now hold run 4).

## Run 4 (final) — program v4 `952b031f…46be`, 2026-08-28

Token program `14b3bc6c…67a8`, token `VEST-TEST`; same creator, beneficiary, beneficiary2 and
nominated authorities as run 3 (see *Accounts*); balances carry over from run 3 (beneficiary
3,767; beneficiary2 1,838; creator 988,395). Steps R4-1…R4-19 repeat run 3 against v4 with
schedules 13–16 in place of 9–12. R4-20 creates schedule 17 — fully linear, start T20 − 600 s,
end T20 − 1 s (already fully vested when created), non-transferable — as a public transaction, and
R4-21 claims it privately with `ClaimToPrivate`. 21 steps: 15 included public transactions,
5 rejected (expected), 1 privacy-preserving claim. Every public claim is signed by the registered
beneficiary holding, so every transaction has a unique hash.

### Accounts

| Schedule | Escrow |
|---|---|
| schedule 13 `3UNdwPKNV8DxjDWe4n1MSwTqEQPqV1PPWgxvCPD6xddu` | `7ZnxdvcesYwLWngYaw9EwZwygNU1QKPzNdvcRM4bdqcv` |
| schedule 14 `BjSkNNmaQLnJBMQzUNxrZ27qvDunkWeBvjAk3tCZ1jRE` | `91RH21kwuj4HnefhbPRusBfoZcLXpCttZXpvhMNe1ets` |
| schedule 15 `FssyedaE9xHB1hPwgmbQpVKDu973W9UggumBCA2XxCtp` | `ACiWrpZ1n7HuSY7DtP2tFQu5hrt3TAEcaPC4bHmX18X9` |
| schedule 16 `2dJwsxhQW2tadDKHMLMK9FUrMm2eN8REpP5nkJQRkgDk` | `DzxumpD4uz9WzfzU31tPgqw3MV2LGrJY6a76yy62sJ82` |
| schedule 17 `DCupLBTdA6zzf7U2oPeL2SQpsaHvJihrXonwFsewx9yM` | `BZv7AChohMo4jGrtQue5Z6qyFu5bZKpfBBp3Xjmoz8zj` |

Escrow PDAs are derived from the v4 ImageID (`cargo run -q -p vesting_program --example
vesting_pdas -- 952b031fecf9e357c76daca7c307e0debb4949d30d419b3c3a1db402a1fa46be <SCHEDULE>`).

### Steps

| R4-1 | create schedule 13: cliff+linear, start T0, cliff T0+120s, end T0+900s, cancelable, transferable, nominated authorities | [`3dd1087f…`](https://explorer.testnet.lez.logos.co/transaction/3dd1087f1d710530f1f73726dbfd877196a6abbcf1dbba0c3a337e116499db1a) | included |
| R4-2 | claim schedule 13 before the cliff (expect rejection: nothing claimable yet) | [`10873489…`](https://explorer.testnet.lez.logos.co/transaction/10873489b7563ea3fa21139dc93b69b2aa666459c701dd99b124eeaab2b9e64f) | rejected — not included (expected) |
| R4-3 | create schedule 14: milestone tranches 300,700 | [`a343e9d0…`](https://explorer.testnet.lez.logos.co/transaction/a343e9d06cac40a2764b5cc14f55fbdf1ef7e62c4218b935ce18d1cf55681950) | included |
| R4-4 | create schedule 15: fully linear, start T7, end T7+1200s, cancelable | [`9d582990…`](https://explorer.testnet.lez.logos.co/transaction/9d582990aa8565a77e24cba828725362f5bc13c679cde985d15db3722b876cd0) | included |
| R4-5 | create schedule 16: cliff+linear over 1h, cancelable (to be made non-cancelable) | [`08d67ec4…`](https://explorer.testnet.lez.logos.co/transaction/08d67ec474a075086bb9181e4ad5d41debcfa342fa3b307254ecdca3e5e7e2d0) | included |
| R4-6 | claim schedule 13 after the cliff (pro-rata, signed by the beneficiary) | [`88f21995…`](https://explorer.testnet.lez.logos.co/transaction/88f21995572c643cf9edc61e1ceb8e095d574eea67861e5c49909111647416eb) | included |
| R4-7 | transfer schedule 13 position to beneficiary2 (signed by current beneficiary) | [`22208f45…`](https://explorer.testnet.lez.logos.co/transaction/22208f456f6a04a54607376c74b1af069b5a1ed861a08b0d3b5efad2b01aa794) | included |
| R4-8 | claim schedule 13 signed by the OLD beneficiary (expect rejection: holding does not match the schedule) | [`d98c90cc…`](https://explorer.testnet.lez.logos.co/transaction/d98c90ccff54c7c181a7ff8e4ba45aae9b03c85d7381fd9fb32f4792ed1732d4) | rejected — not included (expected) |
| R4-9 | signal milestone 0 on schedule 14 | [`a572611f…`](https://explorer.testnet.lez.logos.co/transaction/a572611f9b098226c573da310c5a2aa8b16e49acd50b0338af9ab5815f2b39bd) | included |
| R4-10 | claim schedule 14 (tranche 0 = 300) | [`b687de21…`](https://explorer.testnet.lez.logos.co/transaction/b687de2130bdc90a627f3b6b1c794530d530edc0ed3e3b4792b82bd6eca5e843) | included |
| R4-11 | signal milestone 0 AGAIN on schedule 14 (expect rejection: already signalled) | [`5dc6d1dc…`](https://explorer.testnet.lez.logos.co/transaction/5dc6d1dc974c79bd2be736ac2ec5e18679a1dba60a3d5979b61a431b6b9108bf) | rejected — not included (expected) |
| R4-12 | signal milestone 1 on schedule 14 | [`777464e3…`](https://explorer.testnet.lez.logos.co/transaction/777464e37b0afe97b05e86351356552560e0a4d69e82bd1ce30303506cd35cb5) | included |
| R4-13 | claim-to: schedule 14 tranche 1 (700) paid to beneficiary2's holding, authorised by the beneficiary | [`4039542d…`](https://explorer.testnet.lez.logos.co/transaction/4039542ddbe6052b911a677e9bda6ee3c66ba067b1c7b3424fa7f00a65a53ed3) | included |
| R4-14 | make schedule 16 non-cancelable | [`cc1519d0…`](https://explorer.testnet.lez.logos.co/transaction/cc1519d0c799c904994c76f36a3e07477ba8dfbaf430596cd47f490902a189e1) | included |
| R4-15 | cancel schedule 16 (expect rejection: non-cancelable) | [`bd06608d…`](https://explorer.testnet.lez.logos.co/transaction/bd06608d40d094cedf37466ebc3decf2d4ab4fabbc3e83ada298009e4a84cf8d) | rejected — not included (expected) |
| R4-16 | cancel schedule 15 mid-way (unvested returns to creator, vested stays claimable) | [`b4456101…`](https://explorer.testnet.lez.logos.co/transaction/b44561014cc5370cc273cd3b99b4a92f23c829bf199e10766a4ff0ed28a2e529) | included |
| R4-17 | claim schedule 15 after cancellation (vested-but-unclaimed part) | [`fceba375…`](https://explorer.testnet.lez.logos.co/transaction/fceba375f22277c3bcdc75b76e1ae8769ed7674be831b28b231363433bc96d8f) | included |
| R4-18 | claim schedule 15 again (expect rejection: nothing claimable) | [`69ab83e5…`](https://explorer.testnet.lez.logos.co/transaction/69ab83e53de5800e51d21605cf55408a34db05e42d2a0543d391ec5b6ae46c14) | rejected — not included (expected) |
| R4-19 | claim schedule 13 signed by beneficiary2 after the end (remainder) | [`c44e90ba…`](https://explorer.testnet.lez.logos.co/transaction/c44e90ba2c2e8877ee7a28698bd7aa69bae14fbcf8b3166efd65ef831d5be6b1) | included |
| R4-20 | create schedule 17: fully linear, start T20-600s, end T20-1s (already fully vested), non-transferable | [`7e9e68cd…`](https://explorer.testnet.lez.logos.co/transaction/7e9e68cdec09f9b04dcadc0c6ac49ee6879518ab6d6db62b484288149cc5ec38) | included |
| R4-21 | private claim of schedule 17 (ClaimToPrivate, fresh private destination) | [`3a2828ab…`](https://explorer.testnet.lez.logos.co/transaction/3a2828ab85dcc503959447eb4a8734eaa4ab309fc6ef7f84869727c53b22c72b) | included |

"rejected — not included" means the sequencer executed the transaction, the program panicked
with its deterministic error, and the transaction was left out of the block (LEZ does not
include failing transactions and exposes no client-visible error); the balance checkpoints
after each of those steps are unchanged.

### Step R4-21 — private claim (`ClaimToPrivate`)

Sent by `tools/vesting-private-claim` from the wallet that owns the beneficiary holding
(`21RuJK97…`), as a privacy-preserving transaction: `as_of` = 1787919126620 (now − 60 s at tool
start), destination = a fresh private account initialised in the same transaction under the
beneficiary's own wallet key (`PrivateAuthorizedInit`; the zkVM test
`claim_to_fresh_private_account` uses `PrivateForeignInit` instead), credited by the chained
Token Program transfer. No clock account is declared; the program pins
`timestamp_validity_window = as_of..` and the runtime checks it against the block timestamp at
inclusion. Timing: tool start 12:13:03 UTC (wallet synced to block 27355), inclusion in block
27362 at 12:19:55 UTC — about six minutes from start to inclusion, ~13 GB RAM for the local
succinct proof. The run is recorded in `docs/vesting/testnet-private-claim.tsv`: tx hash,
block 27362, the three public account ids (schedule 17, escrow 17, beneficiary holding),
`as_of`, the init nullifier and the wallet-decrypted balance 1000.

- **Public** (visible in `wallet chain-info transaction --hash 3a2828ab85dcc503959447eb4a8734eaa4ab309fc6ef7f84869727c53b22c72b` and to any observer):
  schedule 17 `DCupLBTd…`, escrow 17 `BZv7AChoh…`, the signing beneficiary holding
  `21RuJK97…`, the claimed amount (1,000, in the schedule state `claimed_amount` /
  `Claimed { amount }` and as the escrow delta 1,000 → 0), and the commitments / nullifiers /
  ciphertexts of the privacy-preserving message. The new note is the 6th of the transaction's
  7 private actions; its initialisation nullifier
  `c260c8e46b9b153039b66464cff5044315ff5e4c7c41f857febcafde1aa93293` is the handle the evidence uses.
- **Hidden**: *which* account (key) holds the tokens, and their later movements — spending the
  note produces an update nullifier that needs the nullifier secret key, and nothing in the
  public data links the note to a wallet. The creator never learns the private account.
- **Inferable, and not claimed as hidden**: the opening balance of the private note. It equals
  the public claim amount (1,000), visible in the schedule state and in the escrow delta.
- **Withheld from the evidence**: the destination account id. LEZ's initialisation nullifier is
  a keyless hash of the account id (`Nullifier::for_account_initialization` =
  SHA256(`"/LEE/v0.3/Nullifier/Initialize/\0"` ‖ id)), and the note's commitment is
  recomputable from (id, owner = token program, balance = the public escrow delta, nonce =
  H(id), data); publishing the id would let anyone pick the note out of the transaction and
  confirm its opening balance. The tool prints the id only to the owner's terminal.
- **Effect**: escrow 17 → 0; the beneficiary's public holding is unchanged; the private balance
  reads 1,000 in the wallet (`private_balance` in the tool's output; recorded as
  `private-note-17 <init nullifier> 1000 wallet-decrypted…` in `testnet-balances.tsv`, not
  readable with `spel inspect`).

### Balance checkpoints (VEST-TEST)

Checkpoints printed by the run driver (`bal` after each mutating step; `e13..e17` are the escrow vaults; the private destination is not a public account and does not appear):

```
BALANCES after-create-13 :: ben=3767 ben2=1838 creator=987395 e13=1000 e14= e15= e16= e17=
BALANCES after-precliff-claim :: ben=3767 ben2=1838 creator=987395 e13=1000 e14= e15= e16= e17=
BALANCES after-creates :: ben=3767 ben2=1838 creator=984395 e13=1000 e14=1000 e15=1000 e16=1000 e17=
BALANCES after-claim-13 :: ben=4234 ben2=1838 creator=984395 e13=533 e14=1000 e15=1000 e16=1000 e17=
BALANCES after-transfer-13 :: ben=4234 ben2=1838 creator=984395 e13=533 e14=1000 e15=1000 e16=1000 e17=
BALANCES after-claim-14a :: ben=4534 ben2=1838 creator=984395 e13=533 e14=700 e15=1000 e16=1000 e17=
BALANCES after-claimto-14b :: ben=4534 ben2=2538 creator=984395 e13=533 e14=0 e15=1000 e16=1000 e17=
BALANCES after-noncancelable-16 :: ben=4534 ben2=2538 creator=984395 e13=533 e14=0 e15=1000 e16=1000 e17=
BALANCES after-cancel-15 :: ben=4534 ben2=2538 creator=984490 e13=533 e14=0 e15=905 e16=1000 e17=
BALANCES after-claim-15 :: ben=5439 ben2=2538 creator=984490 e13=533 e14=0 e15=0 e16=1000 e17=
BALANCES final :: ben=5439 ben2=3071 creator=984490 e13=0 e14=0 e15=0 e16=1000 e17=
BALANCES after-create-17 :: ben=5439 ben2=3071 creator=983490 e13=0 e14=0 e15=0 e16=1000 e17=1000
BALANCES after-private-claim-17 :: ben=5439 ben2=3071 creator=983490 e13=0 e14=0 e15=0 e16=1000 e17=0
```

Final balances (`docs/vesting/testnet-balances.tsv`):

```
vest-beneficiary	21RuJK97VnxdiDA7SYWckkPNyDosnurd8eEjfyiF6ASL	5439
vest-beneficiary2	6C7DWacAjppTMdFNSAS8cv5VEexugyd9W56o2B8p1Xx3	3071
vest-creator	FzmatEJbAggHEzh7KoUeDjngg9jWVAboNrZKyDRmDG71	983490
escrow-13	7ZnxdvcesYwLWngYaw9EwZwygNU1QKPzNdvcRM4bdqcv	0
escrow-14	91RH21kwuj4HnefhbPRusBfoZcLXpCttZXpvhMNe1ets	0
escrow-15	ACiWrpZ1n7HuSY7DtP2tFQu5hrt3TAEcaPC4bHmX18X9	0
escrow-16	DzxumpD4uz9WzfzU31tPgqw3MV2LGrJY6a76yy62sJ82	1000
escrow-17	BZv7AChohMo4jGrtQue5Z6qyFu5bZKpfBBp3Xjmoz8zj	0
private-note-17	c260c8e46b9b153039b66464cff5044315ff5e4c7c41f857febcafde1aa93293	1000	wallet-decrypted, not on chain; identified by init nullifier (see testnet-private-claim.tsv)
```

Reconciliation against run 3's final balances (ben 3767 / ben2 1838 / creator 988,395): creator −5 × 1000 (escrows 13–17) + 95 (unvested returned by the cancel of 15) = **983,490**; ben +467 (pro-rata claim of 13 after the cliff) +300 (tranche 0 of 14) +905 (vested part of 15 after its cancel) = **5439**; ben2 +700 (tranche 1 of 14 via `ClaimTo`) +533 (remainder of 13 after the transfer) = **3071**; escrow-16 stays 1000 (non-cancelable, never claimed); escrow-17 1000 → **0** by the private claim while the public beneficiary holding is unchanged; the private note (init nullifier `c260c8e4…`) decrypts to **1000** in the beneficiary's wallet.

### `scripts/vesting/verify-onchain.sh` output

`testnet-private-claim.tsv`); the earlier "destination not present in public transaction data"
line is gone — see *What `verify-onchain.sh` checks* for why it proved nothing.

```
== transactions (docs/vesting/testnet-transactions.tsv)
✅ ok       3dd1087f1d71…  create schedule 13: cliff+linear, start T0, cliff T0+120s, end T0+900s, cancelable, transferable, nominated authorities
✅ reject   10873489b756…  claim schedule 13 before the cliff (expect rejection: nothing claimable yet)
✅ ok       a343e9d06cac…  create schedule 14: milestone tranches 300,700
✅ ok       9d582990aa85…  create schedule 15: fully linear, start T7, end T7+1200s, cancelable
✅ ok       08d67ec474a0…  create schedule 16: cliff+linear over 1h, cancelable (to be made non-cancelable)
✅ ok       88f21995572c…  claim schedule 13 after the cliff (pro-rata, signed by the beneficiary)
✅ ok       22208f456f6a…  transfer schedule 13 position to beneficiary2 (signed by current beneficiary)
✅ reject   d98c90ccff54…  claim schedule 13 signed by the OLD beneficiary (expect rejection: holding does not match the schedule)
✅ ok       a572611f9b09…  signal milestone 0 on schedule 14
✅ ok       b687de2130bd…  claim schedule 14 (tranche 0 = 300)
✅ reject   5dc6d1dc974c…  signal milestone 0 AGAIN on schedule 14 (expect rejection: already signalled)
✅ ok       777464e37b0a…  signal milestone 1 on schedule 14
✅ ok       4039542ddbe6…  claim-to: schedule 14 tranche 1 (700) paid to beneficiary2's holding, authorised by the beneficiary
✅ ok       cc1519d0c799…  make schedule 16 non-cancelable
✅ reject   bd06608d40d0…  cancel schedule 16 (expect rejection: non-cancelable)
✅ ok       b44561014cc5…  cancel schedule 15 mid-way (unvested returns to creator, vested stays claimable)
✅ ok       fceba375f222…  claim schedule 15 after cancellation (vested-but-unclaimed part)
✅ reject   69ab83e53de5…  claim schedule 15 again (expect rejection: nothing claimable)
✅ ok       c44e90ba2c2e…  claim schedule 13 signed by beneficiary2 after the end (remainder)
✅ ok       7e9e68cdec09…  create schedule 17: fully linear, start T20-600s, end T20-1s (already fully vested), non-transferable
✅ ok       3a2828ab85dc…  private claim of schedule 17 (ClaimToPrivate, fresh private destination)
== final balances
✅ balance vest-beneficiary 5439
✅ balance vest-beneficiary2 3071
✅ balance vest-creator     983490
✅ balance escrow-13        0
✅ balance escrow-14        0
✅ balance escrow-15        0
✅ balance escrow-16        1000
✅ balance escrow-17        0
➖ balance private-note-17  1000 (private; wallet-decrypted, not on chain)
== private claim (docs/vesting/testnet-private-claim.tsv)
✅ private  privacy-preserving tx        3a2828ab85dcc503…
✅ private  public: schedule             DCupLBTdA6zzf7U2…
✅ private  public: escrow               BZv7AChohMo4jGrt…
✅ private  public: beneficiary holding  21RuJK97VnxdiDA7…
✅ private  private: init nullifier      Nullifier(c260c8…
✅ private  validity window              from=1787919126620 to=None
== checked 21 transactions
ALL GOOD
```

Inputs: `docs/vesting/testnet-transactions.tsv`, `docs/vesting/testnet-balances.tsv`,
`docs/vesting/testnet-private-claim.tsv` (run 4).

### What `verify-onchain.sh` checks

For each row of `testnet-transactions.tsv` it asks the sequencer for the hash (`wallet chain-info
transaction --hash`): `ok` rows must be found (included in a block), `reject` rows must not be.
It then reads the fixture holdings with `spel inspect` and compares the balances with
`testnet-balances.tsv` (rows marked `wallet-decrypted` are private notes and are skipped:
their balance is not on chain). For the private claim (R4-21) it reads
`testnet-private-claim.tsv`, dumps the transaction with `wallet chain-info transaction --hash`
and checks that (1) the transaction is privacy-preserving; (2) the three public account ids —
schedule 17, escrow 17, the signing beneficiary holding — appear in its public actions (a
positive control: the dump does show ids when they are public); (3) the init nullifier
`c260c8e4…` appears among its private actions; (4) the output's `timestamp_validity_window` is
`from: Some(as_of), to: None` with the recorded `as_of`. The earlier check that the destination
id was absent from the public data has been dropped: account ids never appear in a
privacy-preserving message by construction, so its absence proved nothing, and the id itself is
no longer recorded anywhere (see R4-21). The script does not decode or verify the public
transactions' contents (instruction, accounts, signers), and the transactions TSV records no
block numbers, so inclusion height is asserted only for the private claim.
