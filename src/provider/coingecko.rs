use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use tracing::{debug, trace};

use super::{CoinPrice, PriceProvider};
use crate::error::{Error, Result};

const BASE_URL: &str = "https://api.coingecko.com/api/v3";

/// CoinGecko price provider -- free public API, no key required.
pub struct CoinGecko {
    client: Client,
}

impl CoinGecko {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("cryptoprice/0.1.0")
            .build()
            .expect("failed to build HTTP client");
        Self { client }
    }

    /// Map common ticker symbols to (CoinGecko API id, display name).
    fn resolve(symbol: &str) -> (String, String) {
        let lower = symbol.to_lowercase();
        let (id, name) = match lower.as_str() {
            "btc" | "bitcoin" => ("bitcoin", "Bitcoin"),
            "eth" | "ethereum" => ("ethereum", "Ethereum"),
            "usdt" | "tether" => ("tether", "Tether"),
            "bnb" => ("binancecoin", "BNB"),
            "sol" | "solana" => ("solana", "Solana"),
            "xrp" | "ripple" => ("ripple", "XRP"),
            "usdc" => ("usd-coin", "USDC"),
            "ada" | "cardano" => ("cardano", "Cardano"),
            "doge" | "dogecoin" => ("dogecoin", "Dogecoin"),
            "dot" | "polkadot" => ("polkadot", "Polkadot"),
            "matic" | "polygon" => ("matic-network", "Polygon"),
            "ltc" | "litecoin" => ("litecoin", "Litecoin"),
            "avax" | "avalanche" => ("avalanche-2", "Avalanche"),
            "link" | "chainlink" => ("chainlink", "Chainlink"),
            "atom" | "cosmos" => ("cosmos", "Cosmos"),
            "uni" | "uniswap" => ("uniswap", "Uniswap"),
            "xlm" | "stellar" => ("stellar", "Stellar"),
            "shib" => ("shiba-inu", "Shiba Inu"),
            "trx" | "tron" => ("tron", "TRON"),
            "ton" => ("the-open-network", "Toncoin"),
            "pepe" => ("pepe", "Pepe"),
            "near" => ("near", "NEAR"),
            "apt" | "aptos" => ("aptos", "Aptos"),
            "arb" | "arbitrum" => ("arbitrum", "Arbitrum"),
            "op" | "optimism" => ("optimism", "Optimism"),
            "sui" => ("sui", "Sui"),
            _ => return (lower.clone(), capitalize(&lower)),
        };
        (id.to_string(), name.to_string())
    }
}

/// CoinGecko `/simple/price` response shape.
/// Example: `{ "bitcoin": { "usd": 50000, "usd_24h_change": 2.5, "usd_market_cap": 9.5e11 } }`
type SimplePrice = HashMap<String, HashMap<String, f64>>;

#[async_trait]
impl PriceProvider for CoinGecko {
    fn name(&self) -> &str {
        "CoinGecko"
    }

    fn id(&self) -> &str {
        "coingecko"
    }

    async fn get_prices(&self, symbols: &[String], currency: &str) -> Result<Vec<CoinPrice>> {
        let resolved: Vec<(String, String)> = symbols.iter().map(|s| Self::resolve(s)).collect();
        let ids_param: String = resolved.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>().join(",");
        let cur = currency.to_lowercase();

        let url = format!(
            "{}/simple/price?ids={}&vs_currencies={}&include_24hr_change=true&include_market_cap=true",
            BASE_URL, ids_param, cur
        );

        debug!(url = %url, "fetching prices from CoinGecko");

        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        let body = resp.text().await?;

        debug!(status = %status, body_len = body.len(), "CoinGecko response");
        trace!(body = %body, "CoinGecko response body");

        if !status.is_success() {
            return Err(Error::Api(format!(
                "CoinGecko returned {}: {}",
                status, body
            )));
        }

        let data: SimplePrice =
            serde_json::from_str(&body).map_err(|e| Error::Parse(format!("CoinGecko JSON: {}", e)))?;

        let change_key = format!("{}_24h_change", cur);
        let cap_key = format!("{}_market_cap", cur);

        let mut results = Vec::new();
        for (i, (cg_id, display_name)) in resolved.iter().enumerate() {
            if let Some(coin_data) = data.get(cg_id.as_str()) {
                let price = coin_data.get(&cur).copied().unwrap_or(0.0);
                results.push(CoinPrice {
                    symbol: symbols[i].to_uppercase(),
                    name: display_name.clone(),
                    price,
                    change_24h: coin_data.get(&change_key).copied(),
                    market_cap: coin_data.get(&cap_key).copied(),
                    currency: cur.to_uppercase(),
                    provider: self.name().to_string(),
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        if results.is_empty() {
            return Err(Error::NoResults);
        }

        Ok(results)
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => {
            let upper: String = c.to_uppercase().collect();
            upper + chars.as_str()
        }
    }
}
