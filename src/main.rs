use ethers::prelude::*;
use std::sync::Arc;
use std::time::{Instant, Duration};
use std::collections::HashMap;
use tokio::time::sleep;
use eyre::Result;

// 1. واجهات العقود الذكية للـ Factory، والـ Pair، والـ ERC20 لجلب الرموز (Symbols)
abigen!(
    IUniswapV2Factory,
    r#"[
        function allPairs(uint256) external view returns (address)
        function allPairsLength() external view returns (uint256)
    ]"#;

    IUniswapV2Pair,
    r#"[
        function token0() external view returns (address)
        function token1() external view returns (address)
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        event Sync(uint112 reserve0, uint112 reserve1)
    ]"#;

    IERC20,
    r#"[
        function symbol() external view returns (string)
    ]"#;
);

// 2. هيكل الحوض مع رموز التوكنات (Symbols)
#[derive(Debug, Clone)]
struct DynamicPool {
    id: usize,
    address: Address,
    token0: Address,
    token1: Address,
    token0_symbol: String,
    token1_symbol: String,
    reserve0: f64,
    reserve1: f64,
    fee: f64,
}

impl DynamicPool {
    // محاكاة معادلة صانع السوق الآلي مع خصم الرسوم
    fn get_amount_out(&self, amount_in: f64, from_token0: bool) -> f64 {
        if amount_in <= 0.0 { return 0.0; }
        
        let (r_in, r_out) = if from_token0 {
            (self.reserve0, self.reserve1)
        } else {
            (self.reserve1, self.reserve0)
        };

        if r_in == 0.0 || r_out == 0.0 { return 0.0; }

        let amount_in_with_fee = amount_in * (1.0 - self.fee);
        let numerator = amount_in_with_fee * r_out;
        let denominator = r_in + amount_in_with_fee;

        numerator / denominator
    }
}

const UNISWAP_V2_FACTORY_BASE: &str = "0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 [Init]: بدء تشغيل محرك المراجحة المتقدم (50 Pools + Symbol Fetching)...");

    let wss_url = std::env::var("QUICKNODE_WSS")
        .unwrap_or_else(|_| "wss://wiser-solemn-bird.base-mainnet.quiknode.pro/f470bcc04e93f882cddaa7f13a58a4672cde33bc".to_string());

    let provider = Provider::<Ws>::connect(&wss_url).await?;
    let client = Arc::new(provider);

    let factory_address: Address = UNISWAP_V2_FACTORY_BASE.parse()?;
    let factory = IUniswapV2Factory::new(factory_address, client.clone());

    println!("🔍 [Factory Query]: جاري الاستعلام عن إجمالي الأحواض...");
    let total_pairs = factory.all_pairs_length().call().await?;
    println!("📊 [Factory Summary]: إجمالي الأحواض المكتشفة: {}", total_pairs);

    let target_pool_count = 50;
    println!("🔄 [Auto Fetch]: جاري جلب تفاصيل أحدث {} حوضاً واستخراج رموز التوكنات...", target_pool_count);

    let mut pools_map: HashMap<Address, DynamicPool> = HashMap::new();
    let mut tracked_addresses: Vec<Address> = Vec::new();

    let start_idx = if total_pairs > U256::from(target_pool_count) {
        total_pairs - U256::from(target_pool_count)
    } else {
        U256::zero()
    };

    let mut pool_counter = 1;
    let mut idx = start_idx;

    while idx < total_pairs && pool_counter <= target_pool_count {
        if let Ok(pair_address) = factory.all_pairs(idx).call().await {
            let pair_contract = IUniswapV2Pair::new(pair_address, client.clone());

            if let (Ok(t0), Ok(t1), Ok((r0, r1, _))) = (
                pair_contract.token_0().call().await,
                pair_contract.token_1().call().await,
                pair_contract.get_reserves().call().await,
            ) {
                // استخراج رموز التوكنات بمرونة (Fallback to ??? if fails)
                let t0_contract = IERC20::new(t0, client.clone());
                let t1_contract = IERC20::new(t1, client.clone());
                
                let sym0 = t0_contract.symbol().call().await.unwrap_or_else(|_| "???".to_string());
                let sym1 = t1_contract.symbol().call().await.unwrap_or_else(|_| "???".to_string());

                let pool_obj = DynamicPool {
                    id: pool_counter,
                    address: pair_address,
                    token0: t0,
                    token1: t1,
                    token0_symbol: sym0.clone(),
                    token1_symbol: sym1.clone(),
                    reserve0: r0 as f64,
                    reserve1: r1 as f64,
                    fee: 0.003,
                };

                pools_map.insert(pair_address, pool_obj);
                tracked_addresses.push(pair_address);

                println!(
                    "  [Pool #{:02}] {}/{} | Address: {:?}",
                    pool_counter, sym0, sym1, pair_address
                );

                pool_counter += 1;
            }
        }
        idx += U256::from(1);

        // ⏱️ تأخير زمني ممتد لـ 200 مللي ثانية لأننا نقوم باستدعاء 4 دوال لكل حوض (لحماية الـ Rate Limit)
        sleep(Duration::from_millis(200)).await;
    }

    println!("\n✅ [Setup Complete]: تم بناء الخريطة لـ {} أحواض وتحديد الرموز بنجاح!", pools_map.len());

    let filter = Filter::new()
        .event("Sync(uint112,uint112)")
        .address(tracked_addresses);

    let mut stream = client.subscribe_logs(&filter).await?;

    println!("⚡ [Live Stream]: جاري بدء المراقبة والمحاكاة اللحظية...\n");

    while let Some(log) = stream.next().await {
        let start_time = Instant::now();

        if let Some(pool) = pools_map.get_mut(&log.address) {
            if let Ok(sync_event) = <SyncFilter as EthEvent>::decode_log(&log.clone().into()) {
                pool.reserve0 = sync_event.reserve_0 as f64;
                pool.reserve1 = sync_event.reserve_1 as f64;

                let exec_time_us = start_time.elapsed().as_micros();

                println!(
                    "⚡ [Exec: {} µs] | {}/{} | R0: {:.0} | R1: {:.0}",
                    exec_time_us, pool.token0_symbol, pool.token1_symbol, pool.reserve0, pool.reserve1
                );

                // 🧠 تشغيل المحاكاة متزامنة مع التحديث اللحظي للسيولة (تبدأ بـ 100 وحدة تقريبية للبحث)
                run_dynamic_triangular_arbitrage(&pools_map, 100.0);
            }
        }
    }

    Ok(())
}

// 🧠 محرك المحاكاة للمراجحة الديناميكية مع عرض رموز التوكنات
fn run_dynamic_triangular_arbitrage(pools: &HashMap<Address, DynamicPool>, start_amount: f64) {
    let pools_vec: Vec<&DynamicPool> = pools.values().collect();

    for p1 in &pools_vec {
        let (start_token, start_symbol, t1_out, t1_sym, amount1) = (
            p1.token0, &p1.token0_symbol, p1.token1, &p1.token1_symbol, p1.get_amount_out(start_amount, true)
        );

        for p2 in &pools_vec {
            if p2.address == p1.address { continue; }
            if p2.token0 != t1_out && p2.token1 != t1_out { continue; }

            let (t2_out, t2_sym, amount2) = if p2.token0 == t1_out {
                (p2.token1, &p2.token1_symbol, p2.get_amount_out(amount1, true))
            } else {
                (p2.token0, &p2.token0_symbol, p2.get_amount_out(amount1, false))
            };

            for p3 in &pools_vec {
                if p3.address == p1.address || p3.address == p2.address { continue; }
                if p3.token0 != t2_out && p3.token1 != t2_out { continue; }

                let final_token = if p3.token0 == t2_out { p3.token1 } else { p3.token0 };
                if final_token != start_token { continue; }

                let final_amount = if p3.token0 == t2_out {
                    p3.get_amount_out(amount2, true)
                } else {
                    p3.get_amount_out(amount2, false)
                };

                let profit = final_amount - start_amount;

                // إذا وجدنا أي مسار مربح بعد خصم الرسوم، يتم طباعته بالرموز
                if profit > 0.0 {
                    println!(
                        "🎯 [Opportunity Found!]:\n   Path: {} -> {} -> {} -> {}\n   Input: {:.4} | Output: {:.4} | Net Profit: +{:.4}\n",
                        start_symbol, t1_sym, t2_sym, start_symbol,
                        start_amount, final_amount, profit
                    );
                }
            }
        }
    }
}
