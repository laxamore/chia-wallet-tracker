# Chia Wallet Tracker

The main purpose of this tool was for me to track my xch cold wallet balance and log all transactions in google sheets using only master public key.

## Config File

The config file format is using toml. The config file is located in ~/.chia-wallet-tracker/config.toml, and the format is as follows:

```toml
# Path to the my custom chia-blockchain which support master public key for read only wallet
chia_blockchain_path = '/opt/chia-blockchain'
wallet_public_key = '9181836e0f5e552f9cc9c25d7a10f73539dae30487f7be2fd9f1a929822917faa2949a5cd6147a09296fee68a9334b3f'
wallet_fingerprint = 4121996123
# Check count is the number of derivation wallets to check
check_count = 100
# The path to the database file
db_path = '$HOME/.chia-wallet-track'
# The name of the database file
db_name = 'wallet_transactions.sqlite'
# Interval to refresh the wallet balance
refresh_interval = 60
# Google sheet config
spreadsheet_id = ''
sheet_name = ''
sheet_range = ''
# Google service account key path as json file
google_service_account_key_path = ''
```

When you run the tool for the first time, it will create the config file for you and you can edit it to your liking.

## Build

To build the tool, you can use the following command:

```bash
cd chia-wallet-tracker
cargo build --release
```
executable file will be located in target/release/chia-wallet-tracker, you can copy it to your bin directory.

```bash
cp target/release/chia-wallet-tracker ~/.local/bin
```

## Custom Chia Blockchain

This tool is using my custom chia-blockchain which support master public key for read only wallet. You can find the source code [here](https://github.com/laxamore/chia-blockchain).

### How to install

To install the custom chia-blockchain, you can follow the default instruction from the official chia-blockchain.

### How to use

**Add wallet using master public key**

```bash
chia keys add_public -p <master_public_key>
```

**Show wallet transactions as json**

```bash
chia wallet get_transactions --print-json
```