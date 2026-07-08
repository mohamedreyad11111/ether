use ethers::prelude::*;
use std::sync::Arc;
use eyre::Result;

const POOL_A: &str = "0xd0b53D9277642d899DF5C87A3966A349A798F224";
const POOL_B: &str = "0x6a77CDeC82EFf6A6A5D273F18C1c27CD3d71A588";

abigen!(
    IUniswapV2Pair,
    r#"[
        event Sync(uint112 reserve0, uint112 reserve1)
    ]"#
);

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 جاري الاتصال بـ QuickNode WSS...");

    let wss_url = std::env::var("QUICKNODE_WSS")
        .unwrap_or_else(|_| "wss://wiser-solemn-bird.base-mainnet.quiknode.pro/f470bcc04e93f882cddaa7f13a58a4672cde33bc".to_string());

    let provider = Provider::<Ws>::connect(&wss_url).await?;
    let client = Arc::new(provider);

    println!("✅ تم الاتصال بنجاح!");

    let pool_a_addr: Address = POOL_A.parse()?;
    let pool_b_addr: Address = POOL_B.parse()?;

    let filter = Filter::new()
        .event("Sync(uint112,uint112)")
        .address(vec![pool_a_addr, pool_b_addr]);

    let mut stream = client.subscribe_logs(&filter).await?;

    while let Some(log) = stream.next().await {
        // فك التشفير الدقيق لتفادي غموض الـ Trait
        if let Ok(sync_event) = <SyncFilter as EthEvent>::decode_log(&log.into()) {
            let r0 = sync_event.reserve_0 as f64;
            let r1 = sync_event.reserve_1 as f64;
            
            let price = if r0 > 0.0 { r1 / r0 } else { 0.0 };
            let pool_name = if log.address == pool_a_addr { "DEX Pool A" } else { "DEX Pool B" };

            println!(
                "⚡ [تحديث سعر]: {} | Reserve0: {:.2} | Reserve1: {:.2} | السعر: {:.6}",
                pool_name, r0, r1, price
            );
        }
    }

    Ok(())
}
