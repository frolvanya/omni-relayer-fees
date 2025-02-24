use alloy::providers::{Provider, ProviderBuilder};
use clap::Parser;
use near_jsonrpc_client::{methods, JsonRpcClient};
use omni_types::ChainKind;

const NEAR_RPC: &str = "https://rpc.mainnet.near.org";
const BASE_RPC: &str = "https://base.llamarpc.com";
const ARB_RPC: &str = "https://arbitrum.llamarpc.com";

const NEAR_FIN_TRANSFER_DEPOSIT: u128 = 600_000_000_000_000_000_000; // https://github.com/Near-One/bridge-sdk-rs/blob/78d96e8ba2c657d3860da46bbc0f02e9a013c1a0/bridge-sdk/bridge-clients/near-bridge-client/src/near_bridge_client.rs#L33

const NEAR_GAS: u128 = 33_220_000_000_000; // https://nearblocks.io/txns/7L6J5qi3Yqabb8i8KrtixN5ujyoswrSzW9egjFuGD8Vv
const BASE_GAS: u128 = 127_652; // https://basescan.org/tx/0xa779997b00a73277bc90dda525e61cf8fb919fd1f2c347cc370f720745e0c21b
const ARB_GAS: u128 = 149_503; // https://arbiscan.io/tx/0x179c58a791909f5e1ac328aa3c810bde916dd3a9070205f6b56758404188fb8d
const SOLANA_GAS: u64 = 103_372; // https://solscan.io/tx/35V7H2BGsyEPw3v2hMzjmQYTC4PwTmu8bY7LiNm2UFMfGhfe86eZPLsKpQFyqsq9vs7HtBrLqFfBUPvLtPW4Qed

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        short,
        long,
        help = "Destination chain (e.g during NEAR to Solana transfer, the destination chain is Solana). Don't specify this argument if you want to calculate fees for all chains"
    )]
    destination_chain: Option<ChainKind>,
    #[arg(short, long, help = "Amount of transfers", default_value = "1000")]
    amount: u128,
    #[arg(
        short,
        long,
        help = "Currency to display fees in",
        default_value = "usd"
    )]
    currency: String,
}

async fn get_token_price(chain: ChainKind, currency: &str) -> f64 {
    let token = match chain {
        ChainKind::Near => "near",
        ChainKind::Eth | ChainKind::Base | ChainKind::Arb => "ethereum",
        ChainKind::Sol => "solana",
    };

    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={token}&vs_currencies={currency}"
    );

    let response = reqwest::get(&url)
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    response[token][currency].as_f64().unwrap()
}

async fn get_near_gas_price() -> u128 {
    let client = JsonRpcClient::connect(NEAR_RPC);
    let request = methods::gas_price::RpcGasPriceRequest { block_id: None };

    client.call(request).await.unwrap().gas_price
}

async fn get_near_fees(amount: u128, currency: &str) {
    let total_near = ((get_near_gas_price().await * NEAR_GAS + NEAR_FIN_TRANSFER_DEPOSIT) * amount)
        as f64
        / 1e24;

    println!(
        "{} transfers to NEAR will burn {:.3} NEARs (approx. {:.3} {})",
        amount,
        total_near,
        total_near * get_token_price(ChainKind::Near, currency).await,
        currency
    );
}

async fn get_evm_gas_price(chain: ChainKind) -> u128 {
    let rpc_http_url = match chain {
        ChainKind::Base => BASE_RPC,
        ChainKind::Arb => ARB_RPC,
        _ => unreachable!("Invalid chain was provided to `get_evm_gas_price` function (only Base and Arb is supported for now)"),
    };

    let client = ProviderBuilder::new().on_http(rpc_http_url.parse().unwrap());

    client.get_gas_price().await.unwrap()
}

async fn get_evm_fees(chain: ChainKind, amount: u128, currency: &str) {
    let gas = match chain {
        ChainKind::Base => BASE_GAS,
        ChainKind::Arb => ARB_GAS,
        _ => unreachable!("Invalid chain was provided to `get_evm_fees` function (only Base and Arb is supported for now)"),
    };

    let total_eth = (get_evm_gas_price(chain).await * gas * amount) as f64 / 1e18;

    println!(
        "{} transfers to {:?} will burn {:.3} ETHs (approx. {:.3} {})",
        amount,
        chain,
        total_eth,
        total_eth * get_token_price(chain, currency).await,
        currency
    );
}

async fn get_solana_fees(amount: u128, currency: &str) {
    let total_sol = (SOLANA_GAS as u128 * amount) as f64 / 1e9;

    println!(
        "{} transfers to Solana will burn {:.6} SOLs (approx. {:.3} {})",
        amount,
        total_sol,
        total_sol * get_token_price(ChainKind::Sol, currency).await,
        currency
    );
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.destination_chain {
        Some(ChainKind::Near) => get_near_fees(args.amount, &args.currency).await,
        Some(ChainKind::Eth) => {
            eprintln!("Fee calculation for Ethereum chain is not supported yet");
        }
        Some(ChainKind::Base) => get_evm_fees(ChainKind::Base, args.amount, &args.currency).await,
        Some(ChainKind::Arb) => get_evm_fees(ChainKind::Arb, args.amount, &args.currency).await,
        Some(ChainKind::Sol) => get_solana_fees(args.amount, &args.currency).await,
        None => {
            get_near_fees(args.amount, &args.currency).await;
            get_evm_fees(ChainKind::Base, args.amount, &args.currency).await;
            get_evm_fees(ChainKind::Arb, args.amount, &args.currency).await;
            get_solana_fees(args.amount, &args.currency).await;
        }
    };
}
