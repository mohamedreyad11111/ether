use ethers::prelude::*;
use std::sync::Arc;
use std::time::Instant;
use std::collections::HashMap;
use eyre::Result;

// 1. تعريف واجهة Smart Contracts للـ Factory والـ Pair ديناميكياً
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
);

// 2. هيكل بيانات الحوض المحدث في الـ Memory
#[derive(Debug, Clone)]
struct DynamicPool {
    id: usize,
    address: Address,
    token0: Address,
    token1: Address,
    reserve0: f64,
    reserve1: f64,
    fee: f64,
}

impl DynamicPool {
    // محاكاة معادلة Constant Product Market Maker (x * y = k)
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

// عنوان Uniswap V2 / SwapBased Factory على شبكة Base Mainnet
const UNISWAP_V2_FACTORY_BASE: &str = "0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 [Init]: بدء تشغيل محرك المراجحة الفائق الديناميكي...");

    let wss_url = std::env::var("QUICKNODE_WSS")
        .unwrap_or_else(|_| "wss://wiser-solemn-bird.base-mainnet.quiknode.pro/f470bcc04e93f882cddaa7f13a58a4672cde33bc".to_string());

    let provider = Provider::<Ws>::connect(&wss_url).await?;
    let client = Arc::new(provider);

    let factory_address: Address = UNISWAP_V2_FACTORY_BASE.parse()?;
    let factory = IUniswapV2Factory::new(factory_address, client.clone());

    // 🔍 3. الاستعلام الديناميكي من الـ Factory مباشرة دون تخمين
    println!("🔍 [Factory Query]: جاري الاستعلام عن إجمالي الأحواض المسجلة في Factory Base...");
    let total_pairs = factory.all_pairs_length().call().await?;
    println!("📊 [Factory Summary]: إجمالي الأحواض المكتشفة في الشبكة: {}", total_pairs);

    let target_pool_count = 20;
    println!("🔄 [Auto Fetch]: جاري جلب تفاصيل أحدث {} حوضاً تلقائياً وتحديد التوكنات...", target_pool_count);

    let mut pools_map: HashMap<Address, DynamicPool> = HashMap::new();
    let mut tracked_addresses: Vec<Address> = Vec::new();

    // جلب أحدث 20 حوضاً من العقد الذكي مباشرة
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

            // جلب عناوين التوكنات والـ Reserves الأولية من البلوكشين
            if let (Ok(t0), Ok(t1), Ok((r0, r1, _))) = (
                pair_contract.token_0().call().await,
                pair_contract.token_1().call().await,
                pair_contract.get_reserves().call().await,
            ) {
                let pool_obj = DynamicPool {
                    id: pool_counter,
                    address: pair_address,
                    token0: t0,
                    token1: t1,
                    reserve0: r0 as f64,
                    reserve1: r1 as f64,
                    fee: 0.003, // 0.3% Fee
                };

                pools_map.insert(pair_address, pool_obj);
                tracked_addresses.push(pair_address);

                println!(
                    "  [Pool #{}] Address: {:?} | Token0: {:?} | Token1: {:?}",
                    pool_counter, pair_address, t0, t1
                );

                pool_counter += 1;
            }
        }
        idx += U256::from(1);
    }

    println!("\n✅ [Setup Complete]: تم بناء الخريطة لـ {} أحواض حقيقية بنجاح!", pools_map.len());

    // ⚡ 4. إعداد الـ Filter بجميع العناوين الديناميكية المكتشفة
    let filter = Filter::new()
        .event("Sync(uint112,uint112)")
        .address(tracked_addresses);

    let mut stream = client.subscribe_logs(&filter).await?;

    println!("⚡ [Live Stream]: جاري بدء المراقبة والمحاكاة اللحظية بدقة فائقة...\n");

    while let Some(log) = stream.next().await {
        let start_time = Instant::now();

        if let Some(pool) = pools_map.get_mut(&log.address) {
            // فك التشفير الدقيق الصارم للـ Log
            if let Ok(sync_event) = <SyncFilter as EthEvent>::decode_log(&log.clone().into()) {
                pool.reserve0 = sync_event.reserve_0 as f64;
                pool.reserve1 = sync_event.reserve_1 as f64;

                let latency_us = start_time.elapsed().as_micros();

                println!(
                    "⚡ [Latency]: {} µs | Pool #{}: {:?} | R0: {:.0} | R1: {:.0}",
                    latency_us, pool.id, pool.address, pool.reserve0, pool.reserve1
                );

                // 🧠 5. تشغيل محاكاة المراجحة الديناميكية بين الأحواض المستكشفة
                run_dynamic_triangular_arbitrage(&pools_map, 1.0);
            }
        }
    }

    Ok(())
}

// 🧠 6. محرك البحث والمحاكاة الديناميكية للمراجحة بين الأحواض المستوردة
fn run_dynamic_triangular_arbitrage(pools: &HashMap<Address, DynamicPool>, start_amount: f64) {
    let pools_vec: Vec<&DynamicPool> = pools.values().collect();

    for p1 in &pools_vec {
        let (start_token, t1_out, amount1) = (p1.token0, p1.token1, p1.get_amount_out(start_amount, true));

        for p2 in &pools_vec {
            if p2.address == p1.address { continue; }
            if p2.token0 != t1_out && p2.token1 != t1_out { continue; }

            let (t2_out, amount2) = if p2.token0 == t1_out {
                (p2.token1, p2.get_amount_out(amount1, true))
            } else {
                (p2.token0, p2.get_amount_out(amount1, false))
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

                if profit > 0.0 {
                    println!(
                        "🎯 [Dynamic Arbitrage Opportunity Found!]:\n   Path: {:?} -> {:?} -> {:?} -> {:?}\n   Input: {:.4} | Expected Output: {:.4} | Profit: +{:.4}\n",
                        start_token, t1_out, t2_out, start_token,
                        start_amount, final_amount, profit
                    );
                }
            }
        }
    }
}
