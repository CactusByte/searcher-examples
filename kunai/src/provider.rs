use std::str::FromStr;

use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};


pub async fn get_tx_async_with_client(
    rpc_client: &RpcClient,
    signature: &str,
    retries: u32,
) -> Result<
    EncodedConfirmedTransactionWithStatusMeta,
    Box<dyn std::error::Error>,
> {
    let sig = Signature::from_str(signature)?;
    let mut backoff = 100;
    for _ in 0..retries {
        match rpc_client
            .get_transaction_with_config(
                &sig,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::JsonParsed),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(1),
                },
            ).await
        {
            Ok(tx) => return Ok(tx),
            Err(e) => {
                println!("Error getting tx: {:?}", e);
                std::thread::sleep(std::time::Duration::from_millis(backoff));
                backoff *= 2;
            }
        }
    }
    Err(format!("could not fetch {}", signature).into())
}

pub async fn get_tx_async(
    signature: &str,
) -> Result<
    EncodedConfirmedTransactionWithStatusMeta,
    Box<dyn std::error::Error>,
> {
    let rpc_client = RpcClient::new("https://mainnet.helius-rpc.com/?api-key=220d6993-c5d4-4c53-864a-c96c7e658a35".to_string());
    let sig = Signature::from_str(signature)?;
    let mut backoff = 100;
    let retries = 5;
    for _ in 0..retries {
        match rpc_client
            .get_transaction_with_config(
                &sig,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::JsonParsed),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(1),
                },
            ).await
        {
            Ok(tx) => return Ok(tx),
            Err(e) => {
                println!("Error getting tx: {:?}", e);
                std::thread::sleep(std::time::Duration::from_millis(backoff));
                backoff *= 2;
            }
        }
    }
    Err(format!("could not fetch {}", signature).into())
}
