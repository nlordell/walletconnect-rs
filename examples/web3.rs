use std::env;
use std::error::Error;
use walletconnect::transport::WalletConnect;
use walletconnect::{qr, Client, Metadata};
use web3::types::TransactionRequest;
use web3::Web3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let client = Client::new(
        "examples-web3",
        Metadata {
            description: "WalletConnect-rs web3 transport example.".into(),
            url: "https://github.com/nlordell/walletconnect-rs".parse()?,
            icons: vec!["https://avatars0.githubusercontent.com/u/4210206".parse()?],
            name: "WalletConnect-rs Web3 Example".into(),
        },
    )?;

    client.ensure_session(qr::print_with_url).await?;

    let wc = WalletConnect::new(client, env::var("INFURA_PROJECT_ID")?)?;
    let web3 = Web3::new(wc);

    let accounts = web3.eth().accounts().await?;
    println!("Connected accounts:");
    for account in &accounts {
        println!(" - {:?}", account);
    }

    let tx = web3
        .eth()
        .send_transaction(TransactionRequest {
            from: accounts[0],
            to: Some("000102030405060708090a0b0c0d0e0f10111213".parse()?),
            value: Some(1_000_000_000_000_000u128.into()),
            ..TransactionRequest::default()
        })
        .await?;

    println!("Transaction sent:\n  https://etherscan.io/tx/{:?}", tx);

    Ok(())
}
