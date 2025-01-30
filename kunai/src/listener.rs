use jito_searcher_client::get_searcher_client_auth;
use reqwest::Client;
use serde_json::json;
use solana_client::nonblocking::{rpc_client::RpcClient, pubsub_client::PubsubClient};
use solana_client::rpc_config::{
    RpcTransactionLogsConfig,
    RpcTransactionLogsFilter,
};
use futures_util::StreamExt;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::EncodableKey;
use solana_sdk::{self, commitment_config::CommitmentConfig};
use tokio::sync::Mutex;
use std::collections::{HashMap, HashSet};
//use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use serenity;

use crate::provider::get_tx_async_with_client;
use crate::pump::{buy_pump_token, fetch_metadata, parse_pump_accounts};

pub const PUMP_FUN_MINT_AUTHORITY: &str = "";

const DISCORD_CHANNEL_ID: u64 = ; // Replace with your channel ID
const DISCORD_BOT_TOKEN: &str = ""; // Replace with your bot token

async fn send_discord_message(token_info: &str, mint: &str, slot: u64) -> Result<(), Box<dyn std::error::Error>> {
    let http = serenity::http::Http::new(DISCORD_BOT_TOKEN);
    
    // Format the message content
    let message_content = format!(
        ":rocket: **New Token Detected!** :moneybag:\n\n\
        **:star2: Token Info:**\n{}\n\n\
        **:key: Mint Address:** `{}`\n\
        **:chart_with_upwards_trend: Slot:** `{}`\n\n\
        :eyes: Keep an eye on this one! :fire:",
        token_info, mint, slot
    );

    // Send the message
    match serenity::model::id::ChannelId(DISCORD_CHANNEL_ID)
        .say(&http, message_content)
        .await
    {
        Ok(_) => println!("Discord message sent successfully!"),
        Err(e) => println!("Failed to send Discord message: {:?}", e),
    }

    Ok(())
}


pub async fn listen_pump_new_coins_discord() -> Result<(), Box<dyn Error>> {
    let rpc_client = Arc::new(RpcClient::new(
        "https://mainnet.helius-rpc.com/?api-key=".to_string(),
    ));

    let client = PubsubClient::new(
        "wss://mainnet.helius-rpc.com/?api-key=",
    )
    .await
    .expect("pubsub client async");

    let (mut notifications, unsub) = client
        .logs_subscribe(
            RpcTransactionLogsFilter::Mentions(vec![PUMP_FUN_MINT_AUTHORITY.to_string()]),
            RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::finalized()),
            },
        )
        .await
        .expect("subscribe to logs");

    println!("Listening for PumpFun events");

    let mut cache = HashMap::<String, bool>::new();

    while let Some(log) = notifications.next().await {
        println!("\n-------------------------------------");
        println!("{:?}", chrono::Local::now());
        let sig = log.value.signature;

        let tx = match get_tx_async_with_client(&rpc_client, &sig, 5).await {
            Ok(tx) => tx,
            Err(e) => {
                println!("Failed to get transaction in time: {:?}", e);
                continue;
            }
        };

        let slot = tx.slot;
        let accounts = match parse_pump_accounts(tx) {
            Ok(accounts) => accounts,
            Err(e) => {
                println!("Error parsing pump accounts: {:?}", e);
                continue;
            }
        };

        let mint = accounts.mint.to_string();
        println!(
            "PumpFun detected: {} (slot: {})",
            mint, slot
        );

        if cache.contains_key(&mint) {
            println!("Already processed {}", mint);
            continue;
        }

        cache.insert(mint.clone(), true);

        /* 

        let metadata = match fetch_metadata(&accounts.mint).await {
            Ok(Some(metadata)) => metadata,
            Ok(None) => {
                println!("Metadata fetch failed or returned None for mint: {}", mint);
                continue;
            }
            Err(e) => {
                println!("Error fetching metadata for {}: {:?}", mint, e);
                continue;
            }
        };

        if !metadata.is_currently_live.unwrap_or(false) {
            println!("Token is not live: {}", mint);
            continue;
        }

        println!("Token is live!");

        let token_info = format!(
            "Name: {}\nSymbol: {}\nWebsite: {}",
            metadata.name.unwrap_or("Unknown".to_string()),
            metadata.symbol.unwrap_or("Unknown".to_string()),
            metadata.website.unwrap_or("None".to_string())
        ); */

        tokio::spawn(async move {
            if let Err(e) = send_discord_message(&mint, &mint, slot).await {
                println!("Error sending Discord message: {:?}", e);
            }
        });
    }

    unsub().await;
    Ok(())
}


pub async fn listen_pump_new_coins() -> Result<(), Box<dyn Error>> {
    let wallet = Arc::new(
        Keypair::read_from_file("C:/Users/user/.config/solana/id.json")
            .expect("read wallet"),
    );

    let auth =
        Arc::new(Keypair::read_from_file("C:/Users/user/.config/solana/).unwrap());

    let searcher_client = Arc::new(Mutex::new(
        get_searcher_client_auth("https://ny.mainnet.block-engine.jito.wtf", &auth)
            .await
            .expect("makes searcher client"),
    ));
    
    let rpc_client = Arc::new(RpcClient::new("https://mainnet.helius-rpc.com/?api-key=".to_string()));
    
    let client = PubsubClient::new("wss://mainnet.helius-rpc.com/?api-key=")
        .await
        .expect("pubsub client async");

    let (mut notifications, unsub) = client
        .logs_subscribe(
            RpcTransactionLogsFilter::Mentions(vec![PUMP_FUN_MINT_AUTHORITY.to_string()]),
            RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::finalized()),
            },
        )
        .await
        .expect("subscribe to logs");

    println!("Listening for PumpFun events");

    let mut cache = HashMap::<String, bool>::new();

    while let Some(log) = notifications.next().await {
        println!("\n-------------------------------------");
        println!("{:?}", chrono::Local::now());
        let sig = log.value.signature;
        
        let tx = match get_tx_async_with_client(&rpc_client, &sig, 5).await {
            Ok(tx) => {
                // println!("{:#?}",tx);
                tx
            }
            Err(_) => {
                println!("did not get tx in time");
                continue;
            }
        };
        let slot = tx.slot;
        let accounts = parse_pump_accounts(tx)?;
        println!(
            "PumpFun shitter: {} (slot: {})",
            accounts.mint.to_string(),
            slot,
        );

        let mint = accounts.mint.to_string();
        if cache.contains_key(&mint) {
            println!("Already bought {} shitter", mint);
            continue;
        }

        cache.insert(mint.clone(), true);

        // sanity check if all fields are populated
        let metadata = fetch_metadata(&accounts.mint)
            .await.unwrap()
            .unwrap();

        // if metadata.website.is_none() {
        //     println!("No website for {}", mint);
        //     continue;
        // }

        // if metadata.twitter.is_none() {
        //     println!("No twitter for {}", mint);
        //     continue;
        // }

        // if metadata.telegram.is_none() {
        //     println!("No telegram for {}", mint);
        //     continue;
        // }

        // // ensure that someone is not passing in the same link for all of the socials
        // let website = metadata.website.unwrap();
        // let twitter = metadata.twitter.unwrap();
        // let telegram = metadata.telegram.unwrap();

        // if website == twitter || website == telegram || twitter == telegram {
        //     println!("Same link for all socials for {}", mint);
        //     continue;
        // }

        let is_live = metadata.is_currently_live.unwrap();

        if !is_live {
            continue;
        }

        println!("is live !!!!!!!!!!!!!!!");

        let wallet_clone = Arc::clone(&wallet);
        let rpc_client_clone = Arc::clone(&rpc_client);
        let mut searcher_client = Arc::clone(&searcher_client);

        tokio::spawn(async move {
            // buy with 0.001 sol
            let result = buy_pump_token(
                &wallet_clone,
                &rpc_client_clone,
                accounts,
                50_000,
                &mut searcher_client,
                true, // use_jito
            )
            .await;
            if let Err(e) = result {
                println!("Error buying pump token: {:?}", e);
            }
        });
    }

    unsub().await;
    Ok(())
}