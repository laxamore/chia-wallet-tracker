use std::process::Command;
use crate::wallet_transactions::RawWalletTransaction;
use super::Config;

pub struct WalletCommands<'a> {
    config: &'a Config
}

impl <'a> WalletCommands<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    pub fn get_wallet_transactions(&self) -> Vec<RawWalletTransaction> {
        let output = Command::new("bash")
            .arg("-c")
            .arg(format!(
                ". {}/activate && chia wallet get_transactions --print-json -f {}",
                self.config.chia_blockchain_path,
                self.config.wallet_fingerprint
            ))
            .output()
            .expect("failed to execute get_transactions command");

        let output = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<Vec<RawWalletTransaction>>(&output).unwrap_or(Vec::new())
    }
}