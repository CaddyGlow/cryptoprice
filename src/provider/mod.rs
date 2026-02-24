mod cache;
pub mod coingecko;
pub mod coinmarketcap;
pub mod frankfurter;
pub mod stooq;
pub mod yahoo;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A single coin's price data returned by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinPrice {
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub change_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub currency: String,
    pub provider: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// A single historical price point for a coin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub price: f64,
}

/// A single ticker search match returned by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerMatch {
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub asset_type: String,
    pub provider: String,
}

/// Sampling interval used when fetching historical chart data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HistoryInterval {
    Auto,
    Hourly,
    Daily,
}

impl HistoryInterval {
    /// Render interval as the CLI-facing lowercase string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Hourly => "hourly",
            Self::Daily => "daily",
        }
    }
}

/// Historical price series for one coin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistory {
    pub symbol: String,
    pub name: String,
    pub currency: String,
    pub provider: String,
    pub points: Vec<PricePoint>,
}

/// Trait implemented by all price data providers.
#[async_trait]
pub trait PriceProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &str;

    /// Short identifier used in CLI flags.
    fn id(&self) -> &str;

    /// Fetch prices for the given coin symbols in the specified fiat currency.
    async fn get_prices(&self, symbols: &[String], currency: &str) -> Result<Vec<CoinPrice>>;

    /// Fetch price history for the given coin symbols.
    ///
    /// Providers that do not support historical data may return a configuration error.
    async fn get_price_history(
        &self,
        _symbols: &[String],
        _currency: &str,
        _days: u32,
        _interval: HistoryInterval,
    ) -> Result<Vec<PriceHistory>> {
        Err(Error::Config(format!(
            "provider '{}' does not support chart mode",
            self.id()
        )))
    }

    /// Fetch price history within an explicit time window.
    ///
    /// Providers that do not support explicit windows may return a configuration error.
    async fn get_price_history_window(
        &self,
        _symbols: &[String],
        _currency: &str,
        _start: Option<chrono::DateTime<chrono::Utc>>,
        _end: chrono::DateTime<chrono::Utc>,
        _interval: HistoryInterval,
    ) -> Result<Vec<PriceHistory>> {
        Err(Error::Config(format!(
            "provider '{}' does not support explicit chart date windows",
            self.id()
        )))
    }

    /// Search provider instruments by symbol/name query.
    ///
    /// Providers that do not support search may return a configuration error.
    async fn search_tickers(&self, _query: &str, _limit: usize) -> Result<Vec<TickerMatch>> {
        Err(Error::Config(format!(
            "provider '{}' does not support ticker search",
            self.id()
        )))
    }
}

/// Build the list of available providers based on configuration.
pub fn available_providers(api_key: Option<String>) -> Vec<Box<dyn PriceProvider>> {
    let cmc_key = api_key.or_else(|| std::env::var("COINMARKETCAP_API_KEY").ok());

    let mut providers: Vec<Box<dyn PriceProvider>> = vec![
        Box::new(coingecko::CoinGecko::new()),
        Box::new(stooq::Stooq::new()),
        Box::new(yahoo::YahooFinance::new()),
    ];
    match cmc_key {
        Some(key) => providers.push(Box::new(coinmarketcap::CoinMarketCap::new(key))),
        None => providers.push(Box::new(coinmarketcap::CoinMarketCap::without_key())),
    }

    providers
}

/// Look up a provider index by its short id.
pub fn get_provider(providers: &[Box<dyn PriceProvider>], id: &str) -> Option<usize> {
    providers
        .iter()
        .position(|p| p.id().eq_ignore_ascii_case(id))
}
