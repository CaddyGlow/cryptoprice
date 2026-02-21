use cryptoprice::error::Error;
use cryptoprice::provider::PriceProvider;
use cryptoprice::provider::coingecko::CoinGecko;
use cryptoprice::provider::coinmarketcap::CoinMarketCap;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn coingecko_provider_fetches_and_parses_mocked_response() {
    let server = MockServer::start().await;
    let response = serde_json::json!({
        "bitcoin": {
            "usd": 50000.0,
            "usd_24h_change": 1.5,
            "usd_market_cap": 999999999.0
        },
        "ethereum": {
            "usd": 3000.0,
            "usd_24h_change": -0.5,
            "usd_market_cap": 500000000.0
        }
    });

    Mock::given(method("GET"))
        .and(path("/api/v3/simple/price"))
        .and(query_param("ids", "bitcoin,ethereum"))
        .and(query_param("vs_currencies", "usd"))
        .and(query_param("include_24hr_change", "true"))
        .and(query_param("include_market_cap", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let provider = CoinGecko::with_base_url(format!("{}/api/v3", server.uri()));
    let symbols = vec!["btc".to_string(), "eth".to_string()];
    let prices = provider.get_prices(&symbols, "usd").await.unwrap();

    assert_eq!(prices.len(), 2);
    assert_eq!(prices[0].symbol, "BTC");
    assert_eq!(prices[0].name, "Bitcoin");
    assert!((prices[0].price - 50000.0).abs() < f64::EPSILON);
    assert_eq!(prices[0].change_24h, Some(1.5));
    assert_eq!(prices[0].market_cap, Some(999999999.0));
    assert_eq!(prices[0].currency, "USD");
    assert_eq!(prices[0].provider, "CoinGecko");

    assert_eq!(prices[1].symbol, "ETH");
    assert_eq!(prices[1].name, "Ethereum");
    assert!((prices[1].price - 3000.0).abs() < f64::EPSILON);
    assert_eq!(prices[1].change_24h, Some(-0.5));
    assert_eq!(prices[1].market_cap, Some(500000000.0));
    assert_eq!(prices[1].currency, "USD");
    assert_eq!(prices[1].provider, "CoinGecko");
}

#[tokio::test]
async fn coingecko_provider_returns_api_error_on_non_success_status() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v3/simple/price"))
        .and(query_param("ids", "bitcoin"))
        .and(query_param("vs_currencies", "usd"))
        .and(query_param("include_24hr_change", "true"))
        .and(query_param("include_market_cap", "true"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
        .mount(&server)
        .await;

    let provider = CoinGecko::with_base_url(format!("{}/api/v3", server.uri()));
    let symbols = vec!["btc".to_string()];
    let result = provider.get_prices(&symbols, "usd").await;

    assert!(matches!(result, Err(Error::Api(ref msg)) if msg.contains("429")));
}

#[tokio::test]
async fn coingecko_provider_returns_parse_error_on_malformed_json() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v3/simple/price"))
        .and(query_param("ids", "bitcoin"))
        .and(query_param("vs_currencies", "usd"))
        .and(query_param("include_24hr_change", "true"))
        .and(query_param("include_market_cap", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not-json"))
        .mount(&server)
        .await;

    let provider = CoinGecko::with_base_url(format!("{}/api/v3", server.uri()));
    let symbols = vec!["btc".to_string()];
    let result = provider.get_prices(&symbols, "usd").await;

    assert!(matches!(result, Err(Error::Parse(ref msg)) if msg.contains("CoinGecko JSON")));
}

#[tokio::test]
async fn coingecko_provider_returns_no_results_when_response_is_empty() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v3/simple/price"))
        .and(query_param("ids", "bitcoin"))
        .and(query_param("vs_currencies", "usd"))
        .and(query_param("include_24hr_change", "true"))
        .and(query_param("include_market_cap", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    let provider = CoinGecko::with_base_url(format!("{}/api/v3", server.uri()));
    let symbols = vec!["btc".to_string()];
    let result = provider.get_prices(&symbols, "usd").await;

    assert!(matches!(result, Err(Error::NoResults)));
}

#[tokio::test]
async fn coinmarketcap_provider_fetches_and_parses_mocked_response() {
    let server = MockServer::start().await;
    let response = serde_json::json!({
        "status": {
            "error_message": null
        },
        "data": {
            "BTC": {
                "name": "Bitcoin",
                "symbol": "BTC",
                "quote": {
                    "USD": {
                        "price": 50000.0,
                        "percent_change_24h": 2.25,
                        "market_cap": 1000000000.0
                    }
                }
            },
            "ETH": {
                "name": "Ethereum",
                "symbol": "ETH",
                "quote": {
                    "USD": {
                        "price": 3000.0,
                        "percent_change_24h": -1.2,
                        "market_cap": 500000000.0
                    }
                }
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/v1/cryptocurrency/quotes/latest"))
        .and(query_param("symbol", "BTC,ETH"))
        .and(query_param("convert", "USD"))
        .and(header("X-CMC_PRO_API_KEY", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let provider =
        CoinMarketCap::with_base_url("test-api-key".to_string(), format!("{}/v1", server.uri()));
    let symbols = vec!["btc".to_string(), "eth".to_string()];
    let prices = provider.get_prices(&symbols, "usd").await.unwrap();

    assert_eq!(prices.len(), 2);
    assert_eq!(prices[0].symbol, "BTC");
    assert_eq!(prices[0].name, "Bitcoin");
    assert!((prices[0].price - 50000.0).abs() < f64::EPSILON);
    assert_eq!(prices[0].change_24h, Some(2.25));
    assert_eq!(prices[0].market_cap, Some(1000000000.0));
    assert_eq!(prices[0].currency, "USD");
    assert_eq!(prices[0].provider, "CoinMarketCap");

    assert_eq!(prices[1].symbol, "ETH");
    assert_eq!(prices[1].name, "Ethereum");
    assert!((prices[1].price - 3000.0).abs() < f64::EPSILON);
    assert_eq!(prices[1].change_24h, Some(-1.2));
    assert_eq!(prices[1].market_cap, Some(500000000.0));
    assert_eq!(prices[1].currency, "USD");
    assert_eq!(prices[1].provider, "CoinMarketCap");
}

#[tokio::test]
async fn coinmarketcap_provider_returns_api_error_on_non_success_status() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/cryptocurrency/quotes/latest"))
        .and(query_param("symbol", "BTC"))
        .and(query_param("convert", "USD"))
        .and(header("X-CMC_PRO_API_KEY", "test-api-key"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&server)
        .await;

    let provider =
        CoinMarketCap::with_base_url("test-api-key".to_string(), format!("{}/v1", server.uri()));
    let symbols = vec!["btc".to_string()];
    let result = provider.get_prices(&symbols, "usd").await;

    assert!(matches!(result, Err(Error::Api(ref msg)) if msg.contains("500")));
}

#[tokio::test]
async fn coinmarketcap_provider_returns_parse_error_on_malformed_json() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/cryptocurrency/quotes/latest"))
        .and(query_param("symbol", "BTC"))
        .and(query_param("convert", "USD"))
        .and(header("X-CMC_PRO_API_KEY", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{broken-json"))
        .mount(&server)
        .await;

    let provider =
        CoinMarketCap::with_base_url("test-api-key".to_string(), format!("{}/v1", server.uri()));
    let symbols = vec!["btc".to_string()];
    let result = provider.get_prices(&symbols, "usd").await;

    assert!(matches!(result, Err(Error::Parse(ref msg)) if msg.contains("CMC JSON")));
}

#[tokio::test]
async fn coinmarketcap_provider_returns_no_results_when_response_has_no_data() {
    let server = MockServer::start().await;
    let response = serde_json::json!({
        "status": {
            "error_message": null
        },
        "data": {}
    });

    Mock::given(method("GET"))
        .and(path("/v1/cryptocurrency/quotes/latest"))
        .and(query_param("symbol", "BTC"))
        .and(query_param("convert", "USD"))
        .and(header("X-CMC_PRO_API_KEY", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let provider =
        CoinMarketCap::with_base_url("test-api-key".to_string(), format!("{}/v1", server.uri()));
    let symbols = vec!["btc".to_string()];
    let result = provider.get_prices(&symbols, "usd").await;

    assert!(matches!(result, Err(Error::NoResults)));
}
