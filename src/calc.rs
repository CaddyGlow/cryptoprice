use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{Error, Result};

/// Recognized fiat currency codes. Prevents false positives on tokens like `1inch` or `3btc`.
const KNOWN_FIAT: &[&str] = &[
    "USD", "EUR", "GBP", "JPY", "CNY", "CAD", "AUD", "CHF", "KRW", "INR", "BRL", "RUB", "TRY",
    "ZAR", "MXN", "SGD", "HKD", "NOK", "SEK", "DKK", "NZD", "PLN", "THB", "TWD", "CZK", "HUF",
    "ILS", "PHP", "MYR", "ARS", "CLP", "COP", "IDR", "SAR", "AED", "NGN", "VND", "PKR", "BDT",
    "EGP",
];

/// A parsed fiat amount from user input (e.g. `3.5EUR`).
#[derive(Debug, Clone)]
pub struct FiatAmount {
    pub amount: f64,
    pub currency: String,
}

/// Result of a fiat-to-crypto conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversion {
    pub from_amount: f64,
    pub from_currency: String,
    pub to_symbol: String,
    pub to_name: String,
    pub to_amount: f64,
    pub rate: f64,
    pub provider: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Try to parse a string like `3.5EUR` or `100usd` into a `FiatAmount`.
///
/// Returns `None` when the input does not match `<number><fiat_code>`, letting
/// the caller fall through to normal price-lookup mode.
pub fn parse_fiat_amount(s: &str) -> Option<FiatAmount> {
    // Find where the alphabetic suffix starts.
    let alpha_start = s.find(|c: char| c.is_ascii_alphabetic())?;
    if alpha_start == 0 {
        return None;
    }

    let (num_part, code_part) = s.split_at(alpha_start);
    let code_upper = code_part.to_uppercase();

    if !KNOWN_FIAT.contains(&code_upper.as_str()) {
        return None;
    }

    let amount: f64 = num_part.parse().ok()?;
    if amount <= 0.0 || !amount.is_finite() {
        return None;
    }

    Some(FiatAmount {
        amount,
        currency: code_upper,
    })
}

/// Returns `true` when `s` (case-insensitive) is a recognized fiat currency code.
pub fn is_known_fiat(s: &str) -> bool {
    KNOWN_FIAT.contains(&s.to_uppercase().as_str())
}

/// Human-readable name for a fiat currency code. Falls back to the code itself.
pub fn fiat_name(code: &str) -> &str {
    match code.to_uppercase().as_str() {
        "USD" => "US Dollar",
        "EUR" => "Euro",
        "GBP" => "British Pound",
        "JPY" => "Japanese Yen",
        "CNY" => "Chinese Yuan",
        "CAD" => "Canadian Dollar",
        "AUD" => "Australian Dollar",
        "CHF" => "Swiss Franc",
        "KRW" => "South Korean Won",
        "INR" => "Indian Rupee",
        "BRL" => "Brazilian Real",
        "RUB" => "Russian Ruble",
        "TRY" => "Turkish Lira",
        "ZAR" => "South African Rand",
        "MXN" => "Mexican Peso",
        "SGD" => "Singapore Dollar",
        "HKD" => "Hong Kong Dollar",
        "NOK" => "Norwegian Krone",
        "SEK" => "Swedish Krona",
        "DKK" => "Danish Krone",
        "NZD" => "New Zealand Dollar",
        "PLN" => "Polish Zloty",
        "THB" => "Thai Baht",
        "TWD" => "New Taiwan Dollar",
        "CZK" => "Czech Koruna",
        "HUF" => "Hungarian Forint",
        "ILS" => "Israeli Shekel",
        "PHP" => "Philippine Peso",
        "MYR" => "Malaysian Ringgit",
        "ARS" => "Argentine Peso",
        "CLP" => "Chilean Peso",
        "COP" => "Colombian Peso",
        "IDR" => "Indonesian Rupiah",
        "SAR" => "Saudi Riyal",
        "AED" => "UAE Dirham",
        "NGN" => "Nigerian Naira",
        "VND" => "Vietnamese Dong",
        "PKR" => "Pakistani Rupee",
        "BDT" => "Bangladeshi Taka",
        "EGP" => "Egyptian Pound",
        _ => code,
    }
}

/// Response shape from `https://api.frankfurter.dev/v1/latest`.
#[derive(Debug, Deserialize)]
struct FrankfurterResponse {
    rates: HashMap<String, f64>,
}

/// Fetch forex rates from the Frankfurter API. Returns a map of target currency -> rate.
///
/// The rate value represents "1 source = rate target" (e.g. 1 USD = 0.85 EUR).
pub async fn fetch_fiat_rates(
    client: &reqwest::Client,
    from: &str,
    to: &[String],
) -> Result<HashMap<String, f64>> {
    let to_param = to.join(",");
    let url = format!(
        "https://api.frankfurter.dev/v1/latest?from={}&to={}",
        from.to_uppercase(),
        to_param.to_uppercase(),
    );

    debug!(url = %url, "fetching forex rates from Frankfurter");

    let resp = client.get(&url).send().await?.error_for_status()?;
    let body: FrankfurterResponse = resp.json().await?;

    debug!(rates = ?body.rates, "received forex rates");

    if body.rates.is_empty() {
        return Err(Error::NoResults);
    }

    Ok(body.rates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_cases() {
        let fa = parse_fiat_amount("3.5EUR").unwrap();
        assert!((fa.amount - 3.5).abs() < f64::EPSILON);
        assert_eq!(fa.currency, "EUR");

        let fa = parse_fiat_amount("100usd").unwrap();
        assert!((fa.amount - 100.0).abs() < f64::EPSILON);
        assert_eq!(fa.currency, "USD");
    }

    #[test]
    fn parse_lowercase_currency() {
        let fa = parse_fiat_amount("42gbp").unwrap();
        assert_eq!(fa.currency, "GBP");
    }

    #[test]
    fn rejects_crypto_symbols() {
        assert!(parse_fiat_amount("1inch").is_none());
        assert!(parse_fiat_amount("3btc").is_none());
    }

    #[test]
    fn rejects_plain_words() {
        assert!(parse_fiat_amount("btc").is_none());
        assert!(parse_fiat_amount("hello").is_none());
    }

    #[test]
    fn rejects_negative_and_zero() {
        assert!(parse_fiat_amount("-5USD").is_none());
        assert!(parse_fiat_amount("0USD").is_none());
    }

    #[test]
    fn rejects_no_number() {
        assert!(parse_fiat_amount("EUR").is_none());
    }

    #[test]
    fn is_known_fiat_works() {
        assert!(is_known_fiat("USD"));
        assert!(is_known_fiat("eur"));
        assert!(is_known_fiat("Gbp"));
        assert!(!is_known_fiat("BTC"));
        assert!(!is_known_fiat("ETH"));
        assert!(!is_known_fiat(""));
    }

    #[test]
    fn fiat_name_known_codes() {
        assert_eq!(fiat_name("USD"), "US Dollar");
        assert_eq!(fiat_name("eur"), "Euro");
        assert_eq!(fiat_name("GBP"), "British Pound");
    }

    #[test]
    fn fiat_name_unknown_returns_code() {
        assert_eq!(fiat_name("XYZ"), "XYZ");
    }

    #[test]
    fn frankfurter_response_parsing() {
        let json = r#"{"amount":1.0,"base":"USD","date":"2026-02-20","rates":{"EUR":0.84983,"GBP":0.74174}}"#;
        let resp: FrankfurterResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.rates.len(), 2);
        assert!((resp.rates["EUR"] - 0.84983).abs() < 1e-6);
        assert!((resp.rates["GBP"] - 0.74174).abs() < 1e-6);
    }
}
