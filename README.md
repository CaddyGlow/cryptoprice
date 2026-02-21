# cryptoprice

A fast Rust CLI to fetch cryptocurrency prices from the terminal.

## Features

- Async provider requests with `tokio` + `reqwest`
- Multiple providers (`coingecko`, `cmc`)
- Human-friendly table output or JSON output for scripts
- Cross-platform release artifacts and Docker publishing via GitHub Actions

## Install

Install with Cargo:

```sh
cargo install --git https://github.com/CaddyGlow/cryptoprice cryptoprice
```

Or build from source:

```sh
cargo build --release
```

Run directly:

```sh
cargo run -- btc eth
```

## Usage

```sh
cryptoprice btc eth
cryptoprice btc --provider cmc
cryptoprice btc eth --json
cryptoprice btc --currency eur
cryptoprice --list-providers
```

## Configuration

- CoinGecko works without an API key.
- CoinMarketCap requires `COINMARKETCAP_API_KEY` (or `--api-key`).

## Development

See `CONTRIBUTING.md` for all development and contribution guidance.

## License

MIT. See `LICENSE`.
