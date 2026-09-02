#!/usr/bin/env bash
# Re-verify the recorded vesting testnet run (run 4, final program) against the sequencer.
#
# Scope — exactly four checks, nothing more:
#   1. every `ok` row of docs/vesting/testnet-transactions.tsv (label <TAB> expect <TAB> tx hash)
#      is found by `wallet chain-info transaction --hash`, i.e. included in a block;
#   2. every `reject` row is NOT found (the sequencer does not include failing transactions);
#   3. the final token balances of the fixture accounts in docs/vesting/testnet-balances.tsv
#      (label <TAB> account id <TAB> balance) match `spel inspect`; rows whose 4th column says
#      `wallet-decrypted` are private accounts and are skipped (their balance is not on chain);
#   4. the private claim recorded in docs/vesting/testnet-private-claim.tsv is on chain as a
#      privacy-preserving transaction whose public data contains exactly the expected public
#      accounts (schedule, escrow, signing beneficiary holding), the note's initialisation
#      nullifier, and a timestamp validity window `as_of..` (no upper bound). The private account
#      id is deliberately not recorded anywhere: LEZ's init nullifier is a keyless hash of it.
# It does NOT decode or verify transaction contents (instruction, accounts, signers). The wallet
# output does carry the block id, but the TSV records no block numbers, so inclusion height is
# not asserted.
#
# Requires `wallet` (logos-execution-zone v0.2.4) and `spel` (logos-co/spel ≥ 1ef0500f) on PATH
# and LEE_WALLET_HOME_DIR pointing at a wallet configured for the same sequencer (any wallet:
# the checks are read-only).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

TSV="${1:-docs/vesting/testnet-transactions.tsv}"
TIDL=artifacts/token-idl.json
fail=0; n=0

check_tx() {
  local label="$1" expect="$2" hash="$3"
  local out; out="$(wallet chain-info transaction --hash "$hash" 2>/dev/null | grep -vE 'Stored' || true)"
  # here-strings, not `echo | grep -q`: a privacy-preserving tx prints ~9 MB and grep -q's early
  # exit would SIGPIPE echo, which `pipefail` reports as a failure
  if grep -q 'Transaction is Some' <<<"$out"; then found=yes; else found=no; fi
  if { [ "$expect" = ok ] && [ "$found" = yes ]; } || { [ "$expect" = reject ] && [ "$found" = no ]; }; then
    printf '✅ %-8s %s  %s\n' "$expect" "${hash:0:12}…" "$label"
  else
    printf '❌ %-8s %s  %s  (found=%s)\n' "$expect" "${hash:0:12}…" "$label" "$found"; fail=1
  fi
}

balance() { spel --idl "$TIDL" inspect "$1" --type TokenHolding 2>/dev/null | grep -oE '"balance": "[0-9]+"' | grep -oE '[0-9]+'; }
check_balance() {
  local label="$1" account="$2" expected="$3"
  local actual; actual="$(balance "$account")"
  if [ "$actual" = "$expected" ]; then printf '✅ balance %-16s %s\n' "$label" "$actual"
  else printf '❌ balance %-16s expected %s got %s\n' "$label" "$expected" "${actual:-?}"; fail=1; fi
}

echo "== transactions ($TSV)"
while IFS=$'\t' read -r label expect hash; do
  [ -z "${hash:-}" ] && continue
  n=$((n+1)); check_tx "$label" "$expect" "$hash"
done < "$TSV"

echo "== final balances"
BAL=docs/vesting/testnet-balances.tsv
while IFS=$'\t' read -r label account expected note; do
  [ -z "${expected:-}" ] && continue
  case "${note:-}" in *wallet-decrypted*) printf '➖ balance %-16s %s (private; wallet-decrypted, not on chain)\n' "$label" "$expected"; continue;; esac
  check_balance "$label" "$account" "$expected"
done < "$BAL"

echo "== private claim (docs/vesting/testnet-private-claim.tsv)"
PRIV=docs/vesting/testnet-private-claim.tsv
if [ -f "$PRIV" ]; then
  pv() { awk -F'\t' -v k="$1" '$1 == k {print $2}' "$PRIV"; }
  priv_tx="$(pv tx)"; out="$(wallet chain-info transaction --hash "$priv_tx" 2>/dev/null | grep -vE 'Stored' || true)"
  pcheck() { # <label> <needle>: the needle must occur in the transaction dump (positive control)
    if grep -qF -- "$2" <<<"$out"; then printf '✅ private  %-28s %s\n' "$1" "${2:0:16}…"
    else printf '❌ private  %-28s %s not found\n' "$1" "${2:0:16}…"; fail=1; fi
  }
  if grep -q 'PrivacyPreserving' <<<"$out"; then printf '✅ private  %-28s %s\n' "privacy-preserving tx" "${priv_tx:0:16}…"
  else printf '❌ private  %-28s %s not found or not privacy-preserving\n' "tx" "${priv_tx:0:16}…"; fail=1; fi
  pcheck "public: schedule" "$(pv schedule)"
  pcheck "public: escrow" "$(pv escrow)"
  pcheck "public: beneficiary holding" "$(pv beneficiary_holding)"
  pcheck "private: init nullifier" "Nullifier($(pv init_nullifier))"
  # Debug output prints `from: Some(<ms>,)`; drop whitespace, newlines and commas before comparing.
  win="$(tr -d ' \n,' <<<"$out" | grep -oE 'timestamp_validity_window:ValidityWindow\{from:Some\([0-9]+\)to:None\}' | head -1 || true)"
  if [ "$win" = "timestamp_validity_window:ValidityWindow{from:Some($(pv as_of))to:None}" ]; then
    printf '✅ private  %-28s from=%s to=None\n' "validity window" "$(pv as_of)"
  else printf '❌ private  %-28s got [%s]\n' "validity window" "$win"; fail=1; fi
else
  echo "(no $PRIV)"
fi

echo "== checked $n transactions"
[ "$fail" = 0 ] && echo "ALL GOOD" || { echo "MISMATCHES FOUND"; exit 1; }
