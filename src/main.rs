use chia_observer_wallet_generator::{derive_wallet, G1Element};
use clap::Parser;
use hex::FromHex;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

mod wallet_cmnds;
mod wallet_transactions;
mod wallet_transactions_save;

use wallet_transactions_save::WalletTransactionsSave;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    save_to_gsheets: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    chia_blockchain_path: String,
    wallet_public_key: String,
    wallet_fingerprint: u32,
    check_count: u32,
    db_path: String,
    db_name: String,
    refresh_interval: u32,
    spreadsheet_id: Option<String>,
    sheet_name: Option<String>,
    sheet_range: Option<String>,
    google_service_account_key_path: Option<String>,
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            chia_blockchain_path: String::from("/opt/chia-blockchain"),
            wallet_public_key: String::from("9181836e0f5e552f9cc9c25d7a10f73539dae30487f7be2fd9f1a929822917faa2949a5cd6147a09296fee68a9334b3f"),
            wallet_fingerprint: 4121996123,
            check_count: 100,
            db_path: format!(
                "{}/.chia-wallet-tracker",
                home::home_dir().unwrap().display()
            ),
            db_name: String::from("wallet_transactions.sqlite"),
            refresh_interval: 60,
            spreadsheet_id: None,
            sheet_name: None,
            sheet_range: None,
            google_service_account_key_path: None,
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mut cfg: Config = confy::load_path(
        format!(
            "{}/.chia-wallet-tracker/config.toml",
            home::home_dir().unwrap().display()
        )
        .as_str(),
    )
    .expect("failed to load config");

    check_configs(&cfg, &args);

    let pk = G1Element::from_bytes(
        &<[u8; 48]>::from_hex(&cfg.wallet_public_key).expect("failed to parse wallet_public_key: "),
    )
    .expect("failed to parse wallet_public_key: ");

    let fingerprint = pk.get_fingerprint();
    if fingerprint != cfg.wallet_fingerprint {
        cfg.wallet_fingerprint = fingerprint;
        confy::store_path(
            format!(
                "{}/.chia-wallet-tracker/config.toml",
                home::home_dir().unwrap().display()
            )
            .as_str(),
            &cfg,
        )
        .expect("failed to store config");
    }

    let mut w_trans_saver = WalletTransactionsSave::new(&cfg);

    let mut wallet_addresses: Vec<String> =
        derive_wallet::generate_multiple_observe_wallet_addresses(&pk, 0, &cfg.check_count);
    let w_cmds = wallet_cmnds::WalletCommands::new(&cfg);

    loop {
        let raw_w_txs = w_cmds.get_wallet_transactions();
        let mut w_txs = wallet_transactions::process_raw_transactions(
            &raw_w_txs,
            &mut wallet_addresses,
            &cfg,
            &pk,
        );
        wallet_transactions::sort_wallet_transactions_by_created_at_time(&mut w_txs);

        w_trans_saver.save_to_db(&w_txs).await.expect("failed to save to db");
        if args.save_to_gsheets {
            w_trans_saver.save_to_googlesheets().await;
        }

        let mut ammount_total: Decimal = Decimal::new(0, 0);
        for tx in w_txs {
            let ammount = tx.chia_amount.unwrap().parse::<Decimal>().unwrap();
            if tx.flow.unwrap() == "incoming" {
                ammount_total += ammount;
            } else {
                ammount_total -= ammount;
            }
        }

        println!(
            "total {} xch amount from total {} checked addresses",
            ammount_total,
            wallet_addresses.len()
        );

        std::thread::sleep(std::time::Duration::from_secs(cfg.refresh_interval.into()));
    }
}

fn check_configs(cfg: &Config, args: &Args) {
    let mut config_ok = true;
    if args.save_to_gsheets {
        if cfg.google_service_account_key_path.is_none() {
            config_ok = false;
            println!("google_service_account_key_path is not set in config.toml file");
        }
        if cfg.sheet_name.is_none() {
            config_ok = false;
            println!("sheet_id is not set in config.toml file");
        }
        if cfg.sheet_range.is_none() {
            config_ok = false;
            println!("sheet_range is not set in config.toml file");
        }
        if cfg.spreadsheet_id.is_none() {
            config_ok = false;
            println!("spreadsheet_id is not set in config.toml file");
        }
    }

    if !config_ok {
        std::process::exit(1);
    }
}
