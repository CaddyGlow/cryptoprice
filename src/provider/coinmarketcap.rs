use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, trace};

use super::{CoinPrice, PriceProvider};
use crate::error::{Error, Result};

const BASE_URL: &str = "https://pro-api.coinmarketcap.com/v1";

/// CoinMarketCap price provider -- requires an API key.
pub struct CoinMarketCap {
    client: Client,
    api_key: String,
}

impl CoinMarketCap {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .user_agent("cryptoprice/0.1.0")
            .build()
            .expect("failed to build HTTP client");
        Self { client, api_key }
    }
}

#[derive(Debug, Deserialize)]
struct CmcCoin {
    name: String,
    symbol: String,
    quote: HashMap<String, CmcQuote>,
}

#[derive(Debug, Deserialize)]
struct CmcQuote {
    price: Option<f64>,
    percent_change_24h: Option<f64>,
    market_cap: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct CmcRawResponse {
    data: HashMap<String, serde_json::Value>,
    status: Option<CmcStatus>,
}

#[derive(Debug, Deserialize)]
struct CmcStatus {
    error_message: Option<String>,
}

#[async_trait]
impl PriceProvider for CoinMarketCap {
    fn name(&self) -> &str {
        "CoinMarketCap"
    }

    fn id(&self) -> &str {
        "cmc"
    }

    async fn get_prices(&self, symbols: &[String], currency: &str) -> Result<Vec<CoinPrice>> {
        let symbols_upper: Vec<String> = symbols.iter().map(|s| s.to_uppercase()).collect();
        let symbols_joined = symbols_upper.join(",");
        let convert = currency.to_uppercase();

        let url = format!(
            "{}/cryptocurrency/quotes/latest?symbol={}&convert={}",
            BASE_URL, symbols_joined, convert
        );

        debug!(url = %url, "fetching prices from CoinMarketCap");

        let resp = self
            .client
            .get(&url)
            .header("X-CMC_PRO_API_KEY", &self.api_key)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        debug!(status = %status, body_len = body.len(), "CoinMarketCap response");
        trace!(body = %body, "CoinMarketCap response body");

        if !status.is_success() {
            return Err(Error::Api(format!(
                "CoinMarketCap returned {}: {}",
                status, body
            )));
        }

        let raw: CmcRawResponse =
            serde_json::from_str(&body).map_err(|e| Error::Parse(format!("CMC JSON: {}", e)))?;

        if let Some(ref st) = raw.status
            && let Some(ref msg) = st.error_message
            && !msg.is_empty()
        {
            return Err(Error::Api(format!("CoinMarketCap: {}", msg)));
        }

        let mut results = Vec::new();
        for sym in &symbols_upper {
            if let Some(val) = raw.data.get(sym.as_str()) {
                // CMC may return a single coin object or an array for duplicate symbols.
                let coin: CmcCoin = if val.is_array() {
                    let coins: Vec<CmcCoin> = serde_json::from_value(val.clone())
                        .map_err(|e| Error::Parse(format!("CMC coin array: {}", e)))?;
                    match coins.into_iter().next() {
                        Some(c) => c,
                        None => continue,
                    }
                } else {
                    serde_json::from_value(val.clone())
                        .map_err(|e| Error::Parse(format!("CMC coin: {}", e)))?
                };

                if let Some(quote) = coin.quote.get(&convert) {
                    results.push(CoinPrice {
                        symbol: coin.symbol.clone(),
                        name: coin.name.clone(),
                        price: quote.price.unwrap_or(0.0),
                        change_24h: quote.percent_change_24h,
                        market_cap: quote.market_cap,
                        currency: convert.clone(),
                        provider: self.name().to_string(),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
        }

        if results.is_empty() {
            return Err(Error::NoResults);
        }

        Ok(results)
    }
}
