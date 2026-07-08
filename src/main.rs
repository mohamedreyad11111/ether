use ethers::prelude::*;
use std::sync::Arc;
use eyre::Result;

// أدرس الأحواض المراد مراقبتها على شبكة Base (أمثلة لأحواض Uniswap V2 / Aerodrome)
// يمكنك تغيير هذه العناوين ببيانات الأحواض التي تريد المقارنة بينها
const POOL_A: &str = "0x88A43bbB192b90082C5D5006D324900C25d6fF88"; // مثال: Hype/WETH Pool
const POOL_B: &str = "0x4c36388bE6F515F3A2E79B5e3D2fF2276C759880"; // مثال: حوض آخر لنفس الزوج

// تعريف الحدث Sync(reserve0, reserve1)
abigen!(
    IUniswapV2Pair,
    r#"[
        event Sync(uint112 reserve0, uint112 reserve1)
    ]"#
);

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 جاري الاتصال بـ QuickNode WSS على شبكة Base...");

    let wss_url = std::env::var("QUICKNODE_WSS")
        .unwrap_or_else(|_| "wss://wiser-solemn-bird.base-mainnet.quiknode.pro/f470bcc04e93f882cddaa7f13a58a4672cde33bc".to_string());

    // إنشاء اتصال Provider عبر WebSocket
    let provider = Provider::<Ws>::connect(&wss_url).await?;
    let client = Arc::new(provider);

    println!("✅ تم الاتصال بنجاح! جاري بدء الاستماع للأحداث والاستجابة فوراً...\n");

    // تحويل العناوين إلى H160
    let pool_a_addr: Address = POOL_A.parse()?;
    let pool_b_addr: Address = POOL_B.parse()?;

    // إنشائ الـ Filter للاستماع لأحداث Sync من الأحواض المحددة
    let filter = Filter::new()
        .event("Sync(uint112,uint112)")
        .address(vec![pool_a_addr, pool_b_addr]);

    let mut stream = client.subscribe_logs(&filter).await?;

    while let Some(log) = stream.next().await {
        // فك تشفير الحدث
        if let Ok(sync_event) = SyncEvent::decode_log(&RawLog {
            topics: log.topics.clone(),
            data: log.data.to_vec(),
        }) {
            let r0 = sync_event.reserve0 as f64;
            let r1 = sync_event.reserve1 as f64;
            
            // حساب السعر البسيط (Reserve1 / Reserve0)
            let price = if r0 > 0.0 { r1 / r0 } else { 0.0 };

            let pool_name = if log.address == pool_a_addr { "DEX Pool A" } else { "DEX Pool B" };

            println!(
                "⚡ [تحديث سعر]: المصدر: {} | Reserve0: {:.2} | Reserve1: {:.2} | السعر المحسوب: {:.6}",
                pool_name, r0, r1, price
            );

            // يمكنك هنا إضافة logic لمقارنة السعر بين الأحواض فوراً لحساب الفروقات (Arbitrage Spread)
        }
    }

    Ok(())
}
