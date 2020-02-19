use std::error::Error;
use std::process;
use walletconnect::{qr, Client, Metadata, Transaction};

fn main() {
    env_logger::init();
    if let Err(err) = futures::executor::block_on(run()) {
        log::error!("{}", err);
        process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let client = Client::new(
        "examples-qr",
        Metadata {
            description: "WalletConnect-rs terminal QR code example".into(),
            url: "https://github.com/nlordell/walletconnect-rs".parse()?,
            icons: vec!["https://avatars0.githubusercontent.com/u/4210206".parse()?],
            name: "WalletConnect-rs QR Example".into(),
        },
    )?;

    let (accounts, _) = client.ensure_session(qr::print).await?;

    println!("Connected accounts:");
    for account in &accounts {
        println!(" - {:?}", account);
    }

    let tx = client
        .send_transaction(Transaction {
            from: accounts[0],
            to: Some("000102030405060708090a0b0c0d0e0f10111213".parse()?),
            value: 1_000_000_000_000_000u128.into(),
            ..Transaction::default()
        })
        .await?;

    println!("Transaction sent:\n  https://etherscan.io/tx/{:?}", tx);

    Ok(())
}
