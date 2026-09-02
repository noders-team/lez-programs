//! Evidence tool: send one `ClaimToPrivate { as_of }` privacy-preserving
//! transaction of the vesting program from the local LEZ wallet.
//!
//! The wallet under `LEE_WALLET_HOME_DIR` must own the beneficiary holding
//! (it signs the claim). The destination is a fresh private account created in
//! the same wallet: only its commitment reaches the chain, never its id.
//! `--dry-run` stops before the wallet is opened: nothing is created, synced,
//! persisted or sent.
//!
//! Usage:
//!   vesting-private-claim --vesting-bin <vesting.bin> --token-bin <token.bin> \
//!     --schedule <id> --escrow <id> --beneficiary-holding <id> \
//!     [--as-of <ms>] [--label <name>] [--expect-image-id <hex>] [--dry-run]
//!
//! Account ids print in base58 and transaction hashes in hex, exactly as the
//! `wallet` CLI prints them; ImageIDs print as the 64-char hex `spel --
//! program-id` shows.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr as _,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context as _, Result};
use clap::Parser;
use common::transaction::LeeTransaction;
use lee::{
    privacy_preserving_transaction::circuit::ProgramWithDependencies, program::Program, Account,
    AccountId, ProgramId,
};
use lee_core::Nullifier;
use token_core::TokenHolding;
use wallet::{
    account::{AccountIdWithPrivacy, Label},
    AccDecodeData, AccountIdentity, WalletCore,
};

/// Default distance of `as_of` behind the local clock. The runtime rejects
/// inclusion before `as_of`, so a local clock slightly ahead of the sequencer
/// must not push `as_of` into the sequencer's future.
const DEFAULT_AS_OF_LAG_MS: u64 = 60_000;

/// Printed right after the destination id. The init nullifier that reaches the
/// chain is `H(prefix || id)` with no key material, so anyone holding the id
/// can link it to the on-chain nullifier.
const DESTINATION_ID_WARNING: &str = "note: the destination id identifies your note (its init \
                                      nullifier is a keyless hash of it) — do not publish it; \
                                      publish the nullifier instead";

#[derive(Parser)]
#[command(about = "Send one ClaimToPrivate { as_of } of the vesting program from the LEZ wallet")]
struct Args {
    /// Docker-built vesting guest (must match the deployed ImageID).
    #[arg(long)]
    vesting_bin: PathBuf,
    /// Docker-built token guest the schedule's `token_program_id` points to.
    #[arg(long)]
    token_bin: PathBuf,
    /// Schedule account (base58 or 0x-hex).
    #[arg(long)]
    schedule: String,
    /// Escrow PDA holding (base58 or 0x-hex).
    #[arg(long)]
    escrow: String,
    /// Registered beneficiary holding (wallet-owned; signs the claim).
    #[arg(long)]
    beneficiary_holding: String,
    /// Timestamp (ms) the vested amount is computed at; default now - 60 s.
    #[arg(long)]
    as_of: Option<u64>,
    /// Wallet label for the fresh private destination account.
    #[arg(long)]
    label: Option<String>,
    /// Abort unless the vesting guest's ImageID equals this 64-char hex.
    #[arg(long)]
    expect_image_id: Option<String>,
    /// Only print the plan; do not open the wallet, create an account, prove or send.
    #[arg(long)]
    dry_run: bool,
}

/// The three public accounts of a private claim, parsed from the CLI.
struct ClaimAccounts {
    schedule: AccountId,
    escrow: AccountId,
    beneficiary: AccountId,
}

impl ClaimAccounts {
    fn parse(args: &Args) -> Result<Self> {
        Ok(Self {
            schedule: parse_account_id(&args.schedule).context("--schedule")?,
            escrow: parse_account_id(&args.escrow).context("--escrow")?,
            beneficiary: parse_account_id(&args.beneficiary_holding)
                .context("--beneficiary-holding")?,
        })
    }

    /// The public part of the declaration, in program order: schedule, escrow,
    /// beneficiary holding (signer).
    fn public_identities(&self) -> [AccountIdentity; 3] {
        [
            AccountIdentity::PublicNoSign(self.schedule),
            AccountIdentity::PublicNoSign(self.escrow),
            AccountIdentity::Public(self.beneficiary),
        ]
    }

    /// Full declaration: the public accounts followed by the destination
    /// (fresh private account), the order the program fixes.
    fn identities(&self, destination: AccountId) -> Vec<AccountIdentity> {
        self.public_identities()
            .into_iter()
            .chain([AccountIdentity::PrivateOwned(destination)])
            .collect()
    }
}

/// Base58 (wallet/spel format) or `0x`-prefixed 32-byte hex.
fn parse_account_id(s: &str) -> Result<AccountId> {
    let Some(hex_part) = s.strip_prefix("0x") else {
        return AccountId::from_str(s).map_err(|e| anyhow!("base58 account id {s}: {e:?}"));
    };
    let bytes = hex::decode(hex_part).with_context(|| format!("hex account id {s}"))?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("account id must be 32 bytes: {s}"))?;
    Ok(AccountId::new(arr))
}

/// 64-char hex of the RISC Zero ImageID (little-endian words), the format
/// `spel -- program-id` prints.
fn image_id_hex(id: &ProgramId) -> String {
    hex::encode(id.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>())
}

fn program_from_file(path: &Path) -> Result<Program> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Program::new(bytes.into()).map_err(|e| anyhow!("program {}: {e}", path.display()))
}

/// Loads both guests, prints their ImageIDs and enforces `--expect-image-id`.
fn load_programs(args: &Args) -> Result<ProgramWithDependencies> {
    let vesting = program_from_file(&args.vesting_bin)?;
    let token = program_from_file(&args.token_bin)?;
    let vesting_hex = image_id_hex(&vesting.id());
    println!("vesting ImageID: {vesting_hex}");
    println!("token   ImageID: {}", image_id_hex(&token.id()));

    if let Some(expected) = &args.expect_image_id {
        let expected = expected.trim_start_matches("0x").to_ascii_lowercase();
        if expected != vesting_hex {
            bail!("vesting ImageID {vesting_hex} does not match --expect-image-id {expected}");
        }
    }
    let token_id = token.id();
    Ok(ProgramWithDependencies::new(
        vesting,
        HashMap::from([(token_id, token)]),
    ))
}

fn unix_now_ms() -> Result<u64> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock before the Unix epoch")?
        .as_millis();
    u64::try_from(millis).context("system clock does not fit in u64 milliseconds")
}

/// `as_of` is the lower bound of the transaction's validity window. The
/// sequencer does not queue a transaction whose window has not opened yet: it
/// is dropped from the mempool, not held until `as_of`. So `as_of` must not
/// be ahead of the sequencer's clock.
fn resolve_as_of(requested: Option<u64>) -> Result<u64> {
    let now_ms = unix_now_ms()?;
    let as_of = requested.unwrap_or_else(|| now_ms.saturating_sub(DEFAULT_AS_OF_LAG_MS));
    if as_of > now_ms {
        bail!(
            "as_of {as_of} is in the future (local now {now_ms}): the sequencer does not queue a \
             transaction whose validity window has not opened -- it is dropped from the mempool, \
             not held until as_of. Pass --as-of <= now (default: now - {DEFAULT_AS_OF_LAG_MS} ms)"
        );
    }
    println!("as_of: {as_of} (local now {now_ms})");
    Ok(as_of)
}

/// Hex of the nullifier the runtime records when `destination` is initialised
/// (`Nullifier::for_account_initialization`): the only on-chain trace of the
/// note, and the value to publish as evidence instead of the id.
fn init_nullifier_hex(destination: &AccountId) -> String {
    hex::encode(Nullifier::for_account_initialization(destination).to_byte_array())
}

/// Creates the private destination in the wallet (optionally labelled) and
/// persists the storage immediately so the key survives a later failure.
fn create_destination(wallet: &mut WalletCore, label: Option<String>) -> Result<AccountId> {
    let label = label.map(Label::from);
    if let Some(label) = &label {
        wallet.storage().check_label_availability(label)?;
    }
    let (destination, chain_index) = wallet.create_new_account_private(None);
    if let Some(label) = label {
        wallet
            .storage_mut()
            .add_label(label, AccountIdWithPrivacy::Private(destination))?;
    }
    wallet.store_persistent_data()?;
    println!("destination: Private/{destination} (chain index {chain_index}; never sent on chain)");
    println!("{DESTINATION_ID_WARNING}");
    println!("init nullifier: {}", init_nullifier_hex(&destination));
    Ok(destination)
}

fn holding_balance(account: &Account) -> Result<u128> {
    match TokenHolding::try_from(&account.data).context("destination data is not a TokenHolding")? {
        TokenHolding::Fungible { balance, .. } => Ok(balance),
        TokenHolding::NftMaster { .. } | TokenHolding::NftPrintedCopy { .. } => {
            bail!("destination holds an NFT, not a fungible balance")
        }
    }
}

/// Mirrors the wallet CLI: locate the note for the destination in the included
/// transaction, decrypt it with the shared secret and cache it in the wallet.
fn record_destination(
    wallet: &mut WalletCore,
    tx: &LeeTransaction,
    destination: AccountId,
    secret: Option<lee::SharedSecretKey>,
) -> Result<()> {
    let private_tx = match tx {
        LeeTransaction::PrivacyPreserving(private_tx) => private_tx,
        LeeTransaction::Public(_) | LeeTransaction::ProgramDeployment(_) => {
            bail!("included transaction is not privacy-preserving")
        }
    };
    let secret =
        secret.ok_or_else(|| anyhow!("wallet returned no shared secret for the destination"))?;
    wallet.decode_insert_privacy_preserving_transaction_results(
        private_tx,
        &[AccDecodeData::Decode(secret, destination)],
    )
}

fn report_private_balance(wallet: &WalletCore, destination: AccountId) {
    match wallet.get_account_private(destination) {
        Some(account) => match holding_balance(&account) {
            Ok(balance) => println!("private_balance: {balance}"),
            Err(e) => println!("private account exists but is not a fungible holding: {e}"),
        },
        None => println!("private account not decrypted locally; rerun `wallet account sync`"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let accounts = ClaimAccounts::parse(&args)?;
    let program = load_programs(&args)?;
    let as_of = resolve_as_of(args.as_of)?;
    let instruction_data =
        Program::serialize_instruction(vesting_core::Instruction::ClaimToPrivate { as_of })
            .map_err(|e| anyhow!("serialize instruction: {e}"))?;

    // Dry run exits before the wallet is opened: opening it syncs the chain and
    // persists storage, and creating the destination would leave a private
    // account behind.
    if args.dry_run {
        println!("destination: <would be created>");
        println!(
            "dry run: would open the wallet, create the private destination, then prove and send \
             ClaimToPrivate {{ as_of: {as_of} }} with {:?} + PrivateOwned(<would be created>)",
            accounts.public_identities()
        );
        return Ok(());
    }

    let mut wallet = WalletCore::from_env()
        .await
        .context("open wallet (LEE_WALLET_HOME_DIR)")?;
    wallet.sync_to_latest_block().await.context("sync wallet")?;
    let destination = create_destination(&mut wallet, args.label)?;
    let identities = accounts.identities(destination);

    println!("proving (succinct, local CPU) and sending; expect several minutes ...");
    let (tx_hash, shared_secrets) = wallet
        .send_privacy_preserving_tx(identities, instruction_data, &program)
        .await
        .context("send private claim")?;
    println!("tx_hash: {tx_hash}");

    let (tx, block) = wallet
        .poll_transaction(tx_hash)
        .await
        .context("poll inclusion")?;
    println!("block: {block}");

    record_destination(
        &mut wallet,
        &tx,
        destination,
        shared_secrets.into_iter().next(),
    )?;
    wallet.store_persistent_data()?;
    report_private_balance(&wallet, destination);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the init-nullifier derivation to the LEZ v0.2.4 domain
    /// (`SHA256("/LEE/v0.3/Nullifier/Initialize/\\0" || id)`) with a synthetic
    /// account id, so a runtime upgrade that changes the domain is caught here.
    /// No real account id is used: the init nullifier is a keyless hash of the
    /// id, so publishing an id alongside its nullifier would deanonymise the note.
    #[test]
    fn init_nullifier_matches_known_vector() {
        let id =
            parse_account_id("0x1111111111111111111111111111111111111111111111111111111111111111")
                .expect("hex account id parses");
        assert_eq!(
            init_nullifier_hex(&id),
            "90806741c13a8a7f8ec9128fbbe8be4c2b75809e176f01c77e77cbbd723af47b"
        );
    }

    #[test]
    fn hex_and_base58_account_ids_agree() {
        let base58 = parse_account_id("29d2S7vB453rNYFdR5Ycwt7y9haRT5fwVwL9zTmBhfV2")
            .expect("base58 account id parses");
        let hex = parse_account_id(&format!("0x{}", hex::encode(base58.value())))
            .expect("hex account id parses");
        assert_eq!(base58, hex);
        assert!(
            parse_account_id("0x00").is_err(),
            "short hex must be rejected"
        );
    }

    #[test]
    fn as_of_in_the_future_is_refused() {
        let err = resolve_as_of(Some(u64::MAX)).expect_err("future as_of must be refused");
        assert!(
            err.to_string().contains("dropped from the mempool"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn default_as_of_lags_the_clock() {
        let now = unix_now_ms().expect("clock");
        let as_of = resolve_as_of(None).expect("default as_of resolves");
        assert!(as_of <= now.saturating_sub(DEFAULT_AS_OF_LAG_MS));
        assert!(
            as_of + DEFAULT_AS_OF_LAG_MS + 5_000 > now,
            "default lag is ~60 s"
        );
    }
}
