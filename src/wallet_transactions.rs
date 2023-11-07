use std::collections::HashMap;

use chia_observer_wallet_generator::G1Element;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::Config;
use crate::derive_wallet::generate_observe_wallet_address;

#[derive(Debug, Deserialize)]
pub struct RawWalletTransaction {
    pub transaction: Option<String>,
    pub status: Option<bool>,
    pub chia_amount: Option<String>,
    pub to_address: Option<String>,
    pub created_at_time: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct WalletTransaction {
    pub transaction: Option<String>,
    pub status: Option<bool>,
    pub chia_amount: Option<String>,
    pub to_address: Option<String>,
    pub created_at_time: Option<String>,
    pub flow: Option<String>,
    pub usd_at_time: Option<Decimal>,
}

pub fn process_raw_transactions(raw_transactions: &Vec<RawWalletTransaction>, wallet_adresses: &mut Vec<String>, config: &Config, pk: &G1Element) -> Vec<WalletTransaction> {
    let mut wallet_transactions_hashmap: HashMap<String, WalletTransaction> = HashMap::new();
    let mut wallet_transactions: Vec<WalletTransaction> = Vec::new();
    let mut new_wallet_adresses: Vec<String> = Vec::new();

    for (i, wallet_address) in wallet_adresses.iter().enumerate() {
        for raw_transaction in raw_transactions {
            if raw_transaction.to_address == Some(wallet_address.clone()) {
                let w_tx = WalletTransaction {
                    transaction: raw_transaction.transaction.clone(),
                    status: raw_transaction.status.clone(),
                    chia_amount: raw_transaction.chia_amount.clone(),
                    to_address: raw_transaction.to_address.clone(),
                    created_at_time: raw_transaction.created_at_time.clone(),
                    flow: Some(String::from("incoming")),
                    usd_at_time: None,
                };
 
                wallet_transactions_hashmap.insert(raw_transaction.transaction.as_ref().unwrap().clone(), w_tx.clone());
                wallet_transactions.push(w_tx);

                let address_len_offset : u32 = (wallet_adresses.len() - i).try_into().unwrap();
                if address_len_offset < config.check_count.try_into().unwrap() {
                    new_wallet_adresses.push(generate_observe_wallet_address(pk, config.check_count + address_len_offset));
                }
            }
            else if !wallet_transactions_hashmap.contains_key(raw_transaction.transaction.as_ref().unwrap()) {
                let w_tx = WalletTransaction {
                    transaction: raw_transaction.transaction.clone(),
                    status: raw_transaction.status.clone(),
                    chia_amount: raw_transaction.chia_amount.clone(),
                    to_address: raw_transaction.to_address.clone(),
                    created_at_time: raw_transaction.created_at_time.clone(),
                    flow: Some(String::from("outgoing")),
                    usd_at_time: None,
                };

                wallet_transactions_hashmap.insert(raw_transaction.transaction.as_ref().unwrap().clone(), w_tx.clone());
                wallet_transactions.push(w_tx);
            }
        }
    };

    if new_wallet_adresses.len() > 0 {
        wallet_adresses.append(&mut new_wallet_adresses);
    }

    wallet_transactions
}

pub fn sort_wallet_transactions_by_created_at_time(wallet_transactions: &mut Vec<WalletTransaction>) {
    wallet_transactions.sort_by(|a, b| {
        let a_created_at_time = a.created_at_time.as_ref().unwrap();
        let b_created_at_time = b.created_at_time.as_ref().unwrap();
        a_created_at_time.partial_cmp(b_created_at_time).unwrap()
    });
}