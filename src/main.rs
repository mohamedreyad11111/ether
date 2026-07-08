use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Default)]
struct MarketData {
    rates: HashMap<String, f64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 [Init]: تشغيل 4 Workers للمراجحة المباشرة والثلاثية على شبكة Base...\n");

    let client = Client::builder()
        .timeout(Duration::from_secs(4))
        .build()?;

    let market_state = Arc::new(RwLock::new(MarketData::default()));

    let weth = "0x4200000000000000000000000000000000000006";
    let usdc = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
    let wbtc = "0x0555E30da8f98308EdB960aa94C0Db47230d2B9c";

    // 🧵 Worker 1: WETH -> USDC
    let c1 = client.clone();
    let s1 = Arc::clone(&market_state);
    let t1 = tokio::spawn(async move {
        fetch_worker(c1, s1, 1, weth, usdc, "1000000000000000000", "WETH_USDC", 1e18, 1e6, 100).await;
    });

    // 🧵 Worker 2: USDC -> WETH
    let c2 = client.clone();
    let s2 = Arc::clone(&market_state);
    let t2 = tokio::spawn(async move {
        fetch_worker(c2, s2, 2, usdc, weth, "2000000000", "USDC_WETH", 1e6, 1e18, 100).await;
    });

    // 🧵 Worker 3: WETH -> WBTC
    let c3 = client.clone();
    let s3 = Arc::clone(&market_state);
    let t3 = tokio::spawn(async move {
        fetch_worker(c3, s3, 3, weth, wbtc, "1000000000000000000", "WETH_WBTC", 1e18, 1e8, 120).await;
    });

    // 🧵 Worker 4: WBTC -> USDC (إكمال المثلث لـ Triangular Arbitrage)
    let c4 = client.clone();
    let s4 = Arc::clone(&market_state);
    let t4 = tokio::spawn(async move {
        fetch_worker(c4, s4, 4, wbtc, usdc, "100000000", "WBTC_USDC", 1e8, 1e6, 120).await;
    });

    let _ = tokio::join!(t1, t2, t3, t4);

    Ok(())
}

async fn fetch_worker(
    client: Client,
    state: Arc<RwLock<MarketData>>,
    worker_id: u8,
    sell_token: &'static str,
    buy_token: &'static str,
    amount: &'static str,
    pair_name: &'static str,
    sell_decimals: f64,
    buy_decimals: f64,
    delay_ms: u64,
) {
    let url = "https://api.0x.org/swap/allowance-holder/price";
    let api_key = "33496247-f998-476f-bc8e-34779b69bd87";
    let mut request_counter = 0;

    loop {
        request_counter += 1;
        let req_start = Instant::now();

        let params = [
            ("chainId", "8453"),
            ("sellToken", sell_token),
            ("buyToken", buy_token),
            ("sellAmount", amount),
        ];

        match client
            .get(url)
            .query(&params)
            .header("0x-api-key", api_key)
            .header("0x-version", "v2")
            .send()
            .await
        {
            Ok(response) => {
                let latency = req_start.elapsed().as_secs_f64() * 1000.0;
                let status = response.status();

                if status.is_success() {
                    if let Ok(body_text) = response.text().await {
                        if let Ok(json) = serde_json::from_str::<Value>(&body_text) {
                            let buy_amount = json["buyAmount"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                            let sell_amount = amount.parse::<f64>().unwrap_or(1.0);

                            let rate = (buy_amount / buy_decimals) / (sell_amount / sell_decimals);

                            println!(
                                "⚡ [W{} | #{:04}]: {} | Latency: {:.1} ms | Rate: {:.6}",
                                worker_id, request_counter, pair_name, latency, rate
                            );

                            {
                                let mut lock = state.write().await;
                                lock.rates.insert(pair_name.to_string(), rate);
                            }

                            // فحص الفرص المباشرة والثلاثية
                            check_all_arbitrage_opportunities(&state).await;
                        }
                    }
                } else if status.as_u16() == 429 {
                    println!("⚠️ [Worker {}]: 429 Rate Limit! انتظار 600ms...", worker_id);
                    tokio::time::sleep(Duration::from_millis(600)).await;
                }
            }
            Err(e) => {
                println!("❌ [Worker {}]: Error: {}", worker_id, e);
            }
        }

        // تنظيم تدفق الطلبات لتفادي حظر 429
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
}

async fn check_all_arbitrage_opportunities(state: &Arc<RwLock<MarketData>>) {
    let lock = state.read().await;

    // 1. المراجحة المباشرة (Direct Arbitrage: WETH -> USDC -> WETH)
    if let (Some(&weth_usdc), Some(&usdc_weth)) = (
        lock.rates.get("WETH_USDC"),
        lock.rates.get("USDC_WETH"),
    ) {
        let return_factor = weth_usdc * usdc_weth;
        let profit_pct = (return_factor - 1.0) * 100.0;

        if return_factor > 1.0 {
            println!("\n🔥 [DIRECT ARBITRAGE FOUND!] 🔥");
            println!("   Factor: {:.6} | Profit: +{:.3}%\n", return_factor, profit_pct);
        }
    }

    // 2. المراجحة الثلاثية (Triangular Arbitrage: WETH -> WBTC -> USDC -> WETH)
    if let (Some(&weth_wbtc), Some(&wbtc_usdc), Some(&usdc_weth)) = (
        lock.rates.get("WETH_WBTC"),
        lock.rates.get("WBTC_USDC"),
        lock.rates.get("USDC_WETH"),
    ) {
        // تحويل 1 WETH إلى WBTC ثم إلى USDC ثم العودة لـ WETH
        let triangular_factor = weth_wbtc * wbtc_usdc * usdc_weth;
        let tri_profit_pct = (triangular_factor - 1.0) * 100.0;

        if triangular_factor > 1.0 {
            println!("\n📐 [TRIANGULAR ARBITRAGE FOUND!] 📐");
            println!("   Path: WETH -> WBTC -> USDC -> WETH");
            println!("   Factor: {:.6} | Profit: +{:.3}%\n", triangular_factor, tri_profit_pct);
        }
    }
}
