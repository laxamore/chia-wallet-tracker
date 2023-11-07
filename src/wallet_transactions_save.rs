extern crate google_sheets4 as sheets4;

use std::collections::HashMap;
use std::hash::Hash;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;
use sheets4::api::ValueRange;
use sheets4::hyper_rustls::HttpsConnector;
use sheets4::{hyper, hyper_rustls, oauth2, Sheets};
use sheets4::Error;

use serde_json::Value;

use crate::{wallet_transactions::WalletTransaction, Config};

pub struct WalletTransactionsSave<'a> {
    config: &'a Config,
    gsheets_hub: Option<Sheets<HttpsConnector<hyper::client::HttpConnector>>>,
    db_connection: Option<rusqlite::Connection>,
    all_coin_paprika_historical_data: Option<AllCoinPaprikaHistoricalData>,
}

impl<'a> WalletTransactionsSave<'a> 
{
    pub fn new(config: &'a Config) -> Self {
        Self { 
            config,
            gsheets_hub: None,
            db_connection: None,
            all_coin_paprika_historical_data: Some(AllCoinPaprikaHistoricalData(HashMap::new())),
        }
    }

    pub async fn save_to_db(&mut self, wallet_transactions: &Vec<WalletTransaction>) -> Result<(), rusqlite::Error> {
        if self.db_connection.is_none() {
            self.create_db_connection()?;
        }

        // Create table if not exists
        let create_table_query = "CREATE TABLE IF NOT EXISTS wallet_transactions (
            `transaction` TEXT PRIMARY KEY,
            status BOOLEAN,
            chia_amount TEXT,
            to_address TEXT,
            created_at_time TEXT,
            flow TEXT,
            usd_at_time TEXT
        )";

        self.db_connection.as_ref().expect("failed to get db_connection")
            .execute(create_table_query, rusqlite::params![])?;

        // Insert values
        let insert_query = "INSERT OR IGNORE INTO wallet_transactions (
            `transaction`,
            status,
            chia_amount,
            to_address,
            created_at_time,
            flow
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

        let query_usd_at_time = "SELECT usd_at_time FROM wallet_transactions WHERE `transaction` = ?1";
        let update_usd_query = "UPDATE wallet_transactions SET usd_at_time = ?1 WHERE `transaction` = ?2";

        for wallet_transaction in wallet_transactions {
            self.db_connection.as_ref().expect("failed to get db_connection")
                .execute(insert_query, rusqlite::params![
                    wallet_transaction.transaction.clone().unwrap_or(String::from("")),
                    wallet_transaction.status.unwrap_or(false),
                    wallet_transaction.chia_amount.clone().unwrap_or(String::from("")),
                    wallet_transaction.to_address.clone().unwrap_or(String::from("")),
                    wallet_transaction.created_at_time.clone().unwrap_or(String::from("")),
                    wallet_transaction.flow.clone().unwrap_or(String::from("")),
                ])?;
                    
            let mut query_stmt = self.db_connection.as_ref().expect("failed to get db_connection")
                .prepare(query_usd_at_time)?;
            let usd_at_time_iter = query_stmt.query_map([wallet_transaction.transaction.clone().unwrap_or(String::from(""))], |row| {
                Ok(CoinGeckoCurrentPrice {
                    usd: Decimal::from_str(&row.get::<_, String>(0).unwrap_or("0.0".to_string())).unwrap()
                })
            })?;

            for usd_at_time in usd_at_time_iter {
                if usd_at_time.unwrap().usd == Decimal::new(0, 0) {
                    self.db_connection.as_ref().expect("failed to get db_connection")
                        .execute(update_usd_query, rusqlite::params![
                            get_xch_to_usd_at_time(
                                wallet_transaction.created_at_time.clone().unwrap_or(String::from("")), 
                                self.all_coin_paprika_historical_data.as_mut().unwrap()
                            ).await.unwrap().to_string(),
                            wallet_transaction.transaction.clone().unwrap_or(String::from("")),
                        ])?;
                }
            }
        }

        Ok(())
    }

    pub async fn save_to_googlesheets(&mut self) {
        if self.gsheets_hub.is_none() {
            // Connect to Google Sheets API
            let secret = oauth2::read_service_account_key(self.config.google_service_account_key_path.as_ref().unwrap())
                .await
                .expect("failed to read credentials.json");
            let client = hyper::Client::builder().build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            );
            let auth = oauth2::ServiceAccountAuthenticator::builder(secret)
                .hyper_client(client.clone())
                .build()
                .await
                .expect("failed to create authenticator");
            self.gsheets_hub = Some(Sheets::new(client, auth));
        }

        // Get wallet transactions
        if self.db_connection.is_none() {
            self.create_db_connection().expect("failed to create db_connection");
        }

        let mut stmt = self.db_connection.as_ref().expect("failed to get db_connection")
            .prepare("SELECT * FROM wallet_transactions").unwrap();
        let wallet_transactions_iter = stmt.query_map(rusqlite::params![], |row| {
            Ok(WalletTransaction {
                transaction: row.get(0).unwrap(),
                status: row.get(1).unwrap(),
                chia_amount: row.get(2).unwrap(),
                to_address: row.get(3).unwrap(),
                created_at_time: row.get(4).unwrap(),
                flow: row.get(5).unwrap(),
                usd_at_time: Some(Decimal::from_str(&row.get::<_, String>(6).unwrap_or("0.0".to_string())).unwrap()),
            })
        }).unwrap();

        let mut wallet_transactions: Vec<WalletTransaction> = Vec::new();
        for wallet_transaction in wallet_transactions_iter {
            wallet_transactions.push(wallet_transaction.unwrap());
        }
        
        // Get spreadsheet ID
        let spreadsheet_id = self
            .config
            .spreadsheet_id
            .as_ref()
            .expect("failed to get spreadsheet_id")
            .clone();

        // Get sheet Name
        let sheet_name = self
            .config
            .sheet_name
            .as_ref()
            .expect("failed to get sheet_id")
            .clone();

        // Get range
        let sheet_range = self
            .config
            .sheet_range
            .as_ref()
            .expect("failed to get range")
            .clone();

        // Get values
        let mut values: Vec<Vec<Value>> = Vec::new();
        for wallet_transaction in wallet_transactions {
            values.push(vec![
                Value::String(
                    wallet_transaction
                        .transaction
                        .clone()
                        .unwrap_or(String::from("")),
                ),
                Value::Bool(wallet_transaction.status.unwrap_or(false)),
                Value::String(
                    wallet_transaction
                        .chia_amount
                        .clone()
                        .unwrap_or(String::from("")),
                ),
                Value::String(
                    wallet_transaction
                        .to_address
                        .clone()
                        .unwrap_or(String::from("")),
                ),
                Value::String(
                    wallet_transaction
                        .created_at_time
                        .clone()
                        .unwrap_or(String::from("")),
                ),
                Value::String(wallet_transaction.flow.clone().unwrap_or(String::from(""))),
                Value::String(wallet_transaction.usd_at_time.unwrap().to_string()),
            ]);
        }

        let sheet_name_range = format!("{}!{}", sheet_name, sheet_range);

        // Update values
        let value_range = ValueRange {
            range: Some(sheet_name_range.clone()),
            major_dimension: Some(String::from("ROWS")),
            values: Some(values),
            ..Default::default()
        };

        let result = self.gsheets_hub.as_ref().expect("failed to get gsheets_hub")
            .spreadsheets()
            .values_update(value_range, &spreadsheet_id, &sheet_name_range)
            .value_input_option("USER_ENTERED")
            .doit()
            .await;

        match result {
            Err(e) => match e {
                // The Error enum provides details about what exactly happened.
                // You can also just use its `Debug`, `Display` or `Error` traits
                Error::HttpError(_)
                | Error::Io(_)
                | Error::MissingAPIKey
                | Error::MissingToken(_)
                | Error::Cancelled
                | Error::UploadSizeLimitExceeded(_, _)
                | Error::Failure(_)
                | Error::BadRequest(_)
                | Error::FieldClash(_)
                | Error::JsonDecodeError(_, _) => println!("{}", e),
            },
            Ok(_) => {}
        }
    }

    fn create_db_connection(&mut self) -> Result<(), rusqlite::Error> {
        // Connect to DB
        let db_connection = rusqlite::Connection::open(format!("{}/{}", self.config.db_path, self.config.db_name))?;
        self.db_connection = Some(db_connection);

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct CoinGeckoCurrentPrice {
    usd: Decimal,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoSimplePriceResponse {
    chia: CoinGeckoCurrentPrice,
}

#[derive(Debug)]
struct CoinPaprikaHistoricalData {
    price: Decimal,
    volume_24h: Decimal,
    market_cap: Decimal,
}

#[derive(Debug, Deserialize)]
struct CoinPaprikaHistoricalResponse {
    timestamp: Option<String>,
    price: Decimal,
    volume_24h: Decimal,
    market_cap: Decimal,
}

#[derive(Debug, Deserialize)]
struct AllCoinPaprikaHistoricalData(#[serde(deserialize_with = "coin_paprika_historical_data_map")] HashMap<String, CoinPaprikaHistoricalData>);

fn coin_paprika_historical_data_map<'de, D>(de: D) -> Result<HashMap<String, CoinPaprikaHistoricalData>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Visitor, SeqAccess};
    struct MapVisitor;
    impl<'de> Visitor<'de> for MapVisitor {
        type Value = HashMap<String, CoinPaprikaHistoricalData>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence of items")
        }

        fn visit_seq<V>(self, mut seq: V) -> Result<HashMap<String, CoinPaprikaHistoricalData>, V::Error>
        where
            V: SeqAccess<'de>,
        {
            let mut map = HashMap::with_capacity(seq.size_hint().unwrap_or(0));

            while let Some(item) = seq.next_element::<CoinPaprikaHistoricalResponse>()? {
                let CoinPaprikaHistoricalResponse {
                    timestamp,
                    price,
                    volume_24h,
                    market_cap,
                } = item;
                match map.entry(timestamp.unwrap()) {
                    std::collections::hash_map::Entry::Occupied(entry) => {
                        return Err(serde::de::Error::custom(format!(
                            "Duplicate entry {}",
                            entry.key()
                        )))
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(CoinPaprikaHistoricalData { price, volume_24h, market_cap })
                    }
                };
            }
            Ok(map)
        }
    }

    de.deserialize_seq(MapVisitor)
}

use chrono::{DateTime, Utc, NaiveDateTime};

async fn get_xch_to_usd_at_time(date: String, all_coin_paprika_historical_data: &mut AllCoinPaprikaHistoricalData) -> Result<Decimal, reqwest::Error> {
    let mut price = Decimal::new(0, 0);
    let timestamp = NaiveDateTime::parse_from_str(&date, "%Y-%m-%d %H:%M:%S").unwrap().timestamp();
    let now = Utc::now().timestamp();

    println!("fetching usd price at {}", date);

    // check if date transaction is just in range of 24 hour ago
    if timestamp > now - 86400 {
        let url = format!("https://api.coingecko.com/api/v3/simple/price?ids=chia&vs_currencies=usd");
        let response = reqwest::get(&url).await?;
        let response_json: CoinGeckoSimplePriceResponse = response.json().await?;
        price = response_json.chia.usd;
    } else {
        // convert timestamp to date yyyy-mm-dd
        let date = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc).format("20%y-%m-%d").to_string();

        if !all_coin_paprika_historical_data.0.contains_key(&format!("{}T00:00:00Z", date)) {
            let url = format!("https://api.coinpaprika.com/v1/tickers/xch-chia-/historical?interval=1d&start={}", date);
            let response = reqwest::get(&url).await?;
            let response_text = response.text().await?;
            *all_coin_paprika_historical_data = serde_json::from_str::<AllCoinPaprikaHistoricalData>(&response_text).unwrap();
        }

        price = all_coin_paprika_historical_data.0.get(&format!("{}T00:00:00Z", date)).unwrap().price;
    }

    Ok(price)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[tokio::test]
    async fn test_get_xch_to_usd_at_time() {
        let mut all_coin_paprika_historical_data = AllCoinPaprikaHistoricalData(HashMap::new());
        let date = String::from("2023-08-31 00:00:00");
        let price = get_xch_to_usd_at_time(date, &mut all_coin_paprika_historical_data).await.unwrap();
        assert_eq!(price, Decimal::from_str("29.54").unwrap());
    }

    #[tokio::test]
    async fn test_save_to_db() {
        use super::*;
        use crate::wallet_transactions::WalletTransaction;
        use crate::Config;

        let mut config = Config::default();
        config.db_path = String::from("/tmp");
        config.db_name = String::from("wallet_transactions_save_test.db");

        let mut wallet_transactions_save = WalletTransactionsSave::new(&config);

        let wallet_transactions = vec![
            WalletTransaction {
                transaction: Some(String::from("test1")),
                status: Some(false),
                chia_amount: Some(String::from("39.39")),
                to_address: Some(String::from("test_address1")),
                created_at_time: Some(String::from("2022-12-30 00:00:00")),
                flow: Some(String::from("39.39")),
                usd_at_time: None,
            },
            WalletTransaction {
                transaction: Some(String::from("test2")),
                status: Some(false),
                chia_amount: Some(String::from("39.39")),
                to_address: Some(String::from("test_address2")),
                created_at_time: Some(String::from("2022-12-30 00:00:00")),
                flow: Some(String::from("39.39")),
                usd_at_time: None,
            },
        ];

        let db_connection = rusqlite::Connection::open(format!("{}/{}", config.db_path, config.db_name)).unwrap();
        db_connection.execute("DROP TABLE IF EXISTS wallet_transactions", rusqlite::params![]).unwrap();

        wallet_transactions_save.save_to_db(&wallet_transactions).await.expect("failed to save to db");

        let mut stmt = db_connection.prepare("SELECT * FROM wallet_transactions").unwrap();
        let wallet_transactions_from_db = stmt.query_map(rusqlite::params![], |row| {
            Ok(WalletTransaction {
                transaction: row.get(0).unwrap(),
                status: row.get(1).unwrap(),
                chia_amount: row.get(2).unwrap(),
                to_address: row.get(3).unwrap(),
                created_at_time: row.get(4).unwrap(),
                flow: row.get(5).unwrap(),
                usd_at_time: None,
            })
        }).unwrap();

        let mut wallet_transactions_from_db_vec = Vec::new();
        for wallet_transaction in wallet_transactions_from_db {
            wallet_transactions_from_db_vec.push(wallet_transaction.unwrap());
        }

        assert_eq!(wallet_transactions, wallet_transactions_from_db_vec);
    }
}