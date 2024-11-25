use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signer::Signer;
use solana_sdk::{
    commitment_config::CommitmentLevel,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    system_program,
    transaction::{Transaction, VersionedTransaction},
};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiMessage, UiParsedMessage,
};
use std::str::FromStr;
use std::sync::Arc;
use std::{error::Error, time::Duration};
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::jito::{send_tx_no_wait, SearcherClient};
use crate::raydium::{create_ata_token_or_not, make_compute_budget_ixs};

pub const BLOXROUTE_ADDRESS: &str = "HWEoBxYs7ssKuudEjzjmpfJVX7Dvi7wescFsVx2L5yoY";
pub const PUMP_GLOBAL_ADDRESS: &str = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";
pub const PUMP_FEE_ADDRESS: &str = "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM";
pub const PUMP_FUN_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const PUMP_FUN_MINT_AUTHORITY: &str = "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM";
pub const EVENT_AUTHORITY: &str = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";
pub const PUMP_BUY_METHOD: [u8; 8] = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];
pub const PUMP_SELL_METHOD: [u8; 8] = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const RENT_PROGRAM: &str = "SysvarRent111111111111111111111111111111111";
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

#[derive(BorshSerialize)]
pub struct PumpFunSwapInstructionData {
    pub method_id: [u8; 8],
    pub token_amount: u64,
    pub lamports: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct PumpAccounts {
    #[serde(
        serialize_with = "pubkey_to_string",
        deserialize_with = "string_to_pubkey"
    )]
    pub mint: Pubkey,
    #[serde(
        serialize_with = "pubkey_to_string",
        deserialize_with = "string_to_pubkey"
    )]
    pub bonding_curve: Pubkey,
    #[serde(
        serialize_with = "pubkey_to_string",
        deserialize_with = "string_to_pubkey"
    )]
    pub associated_bonding_curve: Pubkey,
    #[serde(
        serialize_with = "pubkey_to_string",
        deserialize_with = "string_to_pubkey"
    )]
    pub dev: Pubkey,
    #[serde(
        serialize_with = "pubkey_to_string",
        deserialize_with = "string_to_pubkey"
    )]
    pub metadata: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BondingCurveLayout {
    pub blob1: u64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub blob4: u64,
    pub complete: bool,
}

impl BondingCurveLayout {
    pub const LEN: usize = 8 + 8 + 8 + 8 + 8 + 8 + 1;

    pub fn parse(data: &[u8]) -> Result<Self, std::io::Error> {
        Self::try_from_slice(data)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PumpTokenInfo {
    pub associated_bonding_curve: Option<String>,
    pub bonding_curve: Option<String>,
    pub complete: Option<bool>,
    pub created_timestamp: Option<i64>,
    pub creator: Option<String>,
    pub description: Option<String>,
    pub image_uri: Option<String>,
    pub inverted: Option<bool>,
    pub is_currently_live: Option<bool>,
    pub king_of_the_hill_timestamp: Option<i64>,
    pub last_reply: Option<i64>,
    pub market_cap: Option<f64>,
    pub market_id: Option<String>,
    pub metadata_uri: Option<String>,
    pub mint: Option<String>,
    pub name: Option<String>,
    pub nsfw: Option<bool>,
    pub profile_image: Option<String>,
    pub raydium_pool: Option<String>,
    pub reply_count: Option<i32>,
    pub show_name: Option<bool>,
    pub symbol: Option<String>,
    pub telegram: Option<String>,
    pub total_supply: Option<i64>,
    pub twitter: Option<String>,
    pub usd_market_cap: Option<f64>,
    pub username: Option<String>,
    pub virtual_sol_reserves: Option<i64>,
    pub virtual_token_reserves: Option<i64>,
    pub website: Option<String>,
    pub video_uri: Option<String>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct IPFSMetadata {
    pub name: String,
    pub symbol: String,
    pub description: String,
    pub image: String,
    #[serde(rename = "showName")]
    pub show_name: Option<bool>,
    #[serde(rename = "createdOn")]
    pub created_on: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub website: Option<String>,
}

pub fn parse_pump_accounts(
    tx: EncodedConfirmedTransactionWithStatusMeta,
) -> Result<PumpAccounts, Box<dyn Error>> {
    if let EncodedTransaction::Json(tx) = &tx.transaction.transaction {
        if let UiMessage::Parsed(UiParsedMessage {
            account_keys,
            instructions: _,
            recent_blockhash: _,
            address_table_lookups: _,
        }) = &tx.message
        {
            // println!("Account keys: {:#?}", account_keys);
            if account_keys.len() >= 5 {
                let dev = account_keys[0].pubkey.parse()?;
                let mint = account_keys[1].pubkey.parse()?;
                let bonding_curve = account_keys[3].pubkey.parse()?;
                let associated_bonding_curve = account_keys[4].pubkey.parse()?;
                let metadata = account_keys[5].pubkey.parse()?;

                // println!("dev:{:#?} mint:{:#?} bonding_curve:{:#?} associated_bonding_curve:{:#?} metadata:{:#?} ", dev, mint, bonding_curve, associated_bonding_curve, metadata);

                Ok(PumpAccounts {
                    mint,
                    bonding_curve,
                    associated_bonding_curve,
                    dev,
                    metadata,
                })
            } else {
                Err("Not enough account keys".into())
            }
        } else {
            Err("Not a parsed transaction".into())
        }
    } else {
        Err("Not a JSON transaction".into())
    }
}

async fn _send_tx_standard(
    ixs: Vec<Instruction>,
    wallet: &Keypair,
    rpc_client: &RpcClient,
    owner: Pubkey,
) -> Result<(), Box<dyn Error>> {
    let transaction = VersionedTransaction::from(Transaction::new_signed_with_payer(
        &ixs,
        Some(&owner),
        &[wallet],
        rpc_client.get_latest_blockhash().await?,
    ));
    let res = rpc_client
        .send_transaction_with_config(
            &transaction,
            RpcSendTransactionConfig {
                skip_preflight: true,
                min_context_slot: None,
                preflight_commitment: Some(CommitmentLevel::Processed),
                max_retries: None,
                encoding: None,
            },
        )
        .await;

    match res {
        Ok(sig) => {
            println!("Transaction sent: {}", sig);
            // rpc_client.confirm_transaction_with_spinner(&sig, &rpc_client.get_latest_blockhash().await.unwrap(), CommitmentConfig::confirmed()).await?;
        }
        Err(e) => {
            return Err(e.into());
        }
    }

    Ok(())
}

pub fn make_pump_swap_ix(
    owner: Pubkey,
    mint: Pubkey,
    bonding_curve: Pubkey,
    associated_bonding_curve: Pubkey,
    token_amount: u64,
    lamports: u64,
    ata: Pubkey,
) -> Result<Instruction, Box<dyn Error>> {
    let accounts: [AccountMeta; 12] = [
        AccountMeta::new_readonly(Pubkey::from_str(PUMP_GLOBAL_ADDRESS)?, false),
        AccountMeta::new(Pubkey::from_str(PUMP_FEE_ADDRESS)?, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(associated_bonding_curve, false),
        AccountMeta::new(ata, false),
        AccountMeta::new(owner, true),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(Pubkey::from_str(TOKEN_PROGRAM)?, false),
        AccountMeta::new_readonly(Pubkey::from_str(RENT_PROGRAM)?, false),
        AccountMeta::new_readonly(Pubkey::from_str(EVENT_AUTHORITY)?, false),
        AccountMeta::new_readonly(Pubkey::from_str(PUMP_FUN_PROGRAM)?, false),
    ];

    let data = PumpFunSwapInstructionData {
        method_id: PUMP_BUY_METHOD,
        token_amount,
        lamports,
    };

    Ok(Instruction::new_with_borsh(
        Pubkey::from_str(PUMP_FUN_PROGRAM)?,
        &data,
        accounts.to_vec(),
    ))
}
pub fn _make_buy_ixs(
    owner: Pubkey,
    mint: Pubkey,
    bonding_curve: Pubkey,
    associated_bonding_curve: Pubkey,
    token_amount: u64,
    lamports: u64,
) -> Result<Vec<Instruction>, Box<dyn Error>> {
    let mut ixs = vec![];
    ixs.append(&mut make_compute_budget_ixs(262500, 100000));
    let ata = spl_associated_token_account::get_associated_token_address(&owner, &mint);
    let mut ata_ixs = create_ata_token_or_not(&owner, &mint, &owner, Some(&spl_token::id()));

    ixs.append(&mut ata_ixs);
    ixs.push(make_pump_swap_ix(
        owner,
        mint,
        bonding_curve,
        associated_bonding_curve,
        token_amount,
        lamports,
        ata,
    )?);

    Ok(ixs)
}

pub async fn get_bonding_curve(
    rpc_client: &RpcClient,
    bonding_curve_pubkey: Pubkey,
) -> Result<BondingCurveLayout, Box<dyn Error>> {
    const MAX_RETRIES: u32 = 5;
    const INITIAL_DELAY_MS: u64 = 200;
    let mut retries = 0;
    let mut delay = Duration::from_millis(INITIAL_DELAY_MS);

    loop {
        match rpc_client
            .get_account_with_config(
                &bonding_curve_pubkey,
                RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(CommitmentConfig::processed()),
                    data_slice: None,
                    min_context_slot: None,
                },
            )
            .await
        {
            Ok(res) => {
                if let Some(account) = res.value {
                    // Convert Vec<u8> to [u8; 49]
                    let data_length = account.data.len();
                    let data: [u8; 49] = account
                        .data
                        .try_into()
                        .map_err(|_| format!("Invalid data length: {}", data_length))?;

                    println!("Raw bytes: {:?}", data);

                    let layout = BondingCurveLayout {
                        blob1: u64::from_le_bytes(data[0..8].try_into()?),
                        virtual_token_reserves: u64::from_le_bytes(data[8..16].try_into()?),
                        virtual_sol_reserves: u64::from_le_bytes(data[16..24].try_into()?),
                        real_token_reserves: u64::from_le_bytes(data[24..32].try_into()?),
                        real_sol_reserves: u64::from_le_bytes(data[32..40].try_into()?),
                        blob4: u64::from_le_bytes(data[40..48].try_into()?),
                        complete: data[48] != 0,
                    };

                    println!("Parsed BondingCurveLayout: {:?}", layout);
                    return Ok(layout);
                } else {
                    if retries >= MAX_RETRIES {
                        println!("Max retries reached. Account not found.");
                        return Err("Account not found after max retries".into());
                    }
                    println!(
                        "Attempt {} failed: Account not found. Retrying in {:?}...",
                        retries + 1,
                        delay
                    );
                    sleep(delay).await;
                    retries += 1;
                    delay = Duration::from_millis(INITIAL_DELAY_MS * 2u64.pow(retries));
                    continue;
                }
            }
            Err(e) => {
                if retries >= MAX_RETRIES {
                    println!("Max retries reached. Last error: {}", e);
                    return Err(format!("Max retries reached. Last error: {}", e).into());
                }
                println!(
                    "Attempt {} failed: {}. Retrying in {:?}...",
                    retries + 1,
                    e,
                    delay
                );
                sleep(delay).await;
                retries += 1;
                delay = Duration::from_millis(INITIAL_DELAY_MS * 2u64.pow(retries));
            }
        }
    }
}

pub fn get_token_amount(
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    real_token_reserves: u64,
    lamports: u64,
) -> Result<u64, Box<dyn std::error::Error>> {
    let virtual_sol_reserves = virtual_sol_reserves as u128;
    let virtual_token_reserves = virtual_token_reserves as u128;
    let amount_in = lamports as u128;

    // Calculate reserves_product carefully to avoid overflow
    let reserves_product = virtual_sol_reserves
        .checked_mul(virtual_token_reserves)
        .ok_or("Overflow in reserves product calculation")?;

    let new_virtual_sol_reserve = virtual_sol_reserves
        .checked_add(amount_in)
        .ok_or("Overflow in new virtual SOL reserve calculation")?;

    let new_virtual_token_reserve = reserves_product
        .checked_div(new_virtual_sol_reserve)
        .ok_or("Division by zero or overflow in new virtual token reserve calculation")?
        .checked_add(1)
        .ok_or("Overflow in new virtual token reserve calculation")?;

    let amount_out = virtual_token_reserves
        .checked_sub(new_virtual_token_reserve)
        .ok_or("Underflow in amount out calculation")?;

    let final_amount_out = std::cmp::min(amount_out, real_token_reserves as u128);

    Ok(final_amount_out as u64)
}

pub async fn buy_pump_token(
    wallet: &Keypair,
    rpc_client: &RpcClient,
    pump_accounts: PumpAccounts,
    lamports: u64,
    searcher_client: &mut Arc<Mutex<SearcherClient>>,
    use_jito: bool,
) -> Result<(), Box<dyn Error>> {
    let owner = wallet.pubkey();

    let bonding_curve = get_bonding_curve(rpc_client, pump_accounts.bonding_curve).await?;
    let token_amount = get_token_amount(
        bonding_curve.virtual_sol_reserves,
        bonding_curve.virtual_token_reserves,
        bonding_curve.real_token_reserves,
        lamports,
    )?;

    // apply slippage in a stupid manner
    let token_amount = (token_amount as f64 * 0.9) as u64;

    println!("buying {}", token_amount);

    let mut ixs = _make_buy_ixs(
        owner,
        pump_accounts.mint,
        pump_accounts.bonding_curve,
        pump_accounts.associated_bonding_curve,
        token_amount,
        lamports,
    )?;

    // send transaction with jito
    // 0.0001 sol tip
    if use_jito {
        let tip: u64 = 10_000;
        let mut searcher_client = searcher_client.lock().await;
        send_tx_no_wait(&mut ixs, tip, wallet, &mut searcher_client, rpc_client).await?;
    } else {
        _send_tx_standard(ixs, wallet, rpc_client, owner).await?;
    }

    // send the tx with spinner
    // let res = rpc_client
    //     .send_and_confirm_transaction_with_spinner_and_config(
    //         &transaction,
    //         CommitmentConfig::processed(),
    //         RpcSendTransactionConfig {
    //             encoding: Some(UiTransactionEncoding::Base64),
    //             skip_preflight: true,
    //             max_retries: None,
    //             preflight_commitment: None,
    //             min_context_slot: None,
    //         },
    //     )
    //     .await;
    //
    // send the transaction without spinner

    Ok(())
}

pub async fn fetch_metadata(mint: &Pubkey) -> Result<Option<PumpTokenInfo>, Box<dyn Error>> {
    const MAX_RETRIES: u32 = 3;
    const INITIAL_DELAY_MS: u64 = 1000;

    let mut retry_count = 0;
    let mut delay_ms = INITIAL_DELAY_MS;

    while retry_count < MAX_RETRIES {
        match fetch_metadata_inner(mint).await {
            Ok(metadata) => {
                println!("Successfully fetched metadata for mint: {}", mint);
                return Ok(Some(metadata));
            }
            Err(e) => {
                println!(
                    "Attempt {} to fetch metadata failed: {}. Retrying in {} ms...",
                    retry_count + 1,
                    e,
                    delay_ms
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                retry_count += 1;
                delay_ms *= 2; // Exponential backoff
            }
        }
    }

    println!("Failed to fetch metadata after {} retries.", MAX_RETRIES);
    Ok(None) // Return None if all retries fail
}

async fn fetch_metadata_inner(mint: &Pubkey) -> Result<PumpTokenInfo, Box<dyn Error>> {
    let url = format!("https://frontend-api.pump.fun/coins/{}", mint);
    println!("Fetching metadata from URL: {}", url);

    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        let metadata = response.json::<PumpTokenInfo>().await?;
        Ok(metadata)
    } else {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_else(|_| "No body".to_string());
        println!("HTTP Error: {}. Body: {}", status, error_body);

        // Return the error as a string for higher-level handling
        Err(format!("HTTP Error: {}. Body: {}", status, error_body).into())
    }
}



pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1000000000.0
}

pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1000000000.0) as u64
}

/// Helper function for pubkey serialize
pub fn pubkey_to_string<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&pubkey.to_string())
}

/// Helper function for pubkey deserialize
pub fn string_to_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <std::string::String as Deserialize>::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

pub fn max(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

pub fn string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}