use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// 1. هيكل الذاكرة المشتركة لتخزين الأسعار الحالية بين الخيوط المتوازية
#[derive(Default)]
struct MarketData {
    // يخزن السعر كنسبة تحويل (مثلاً: 1 WETH = 1730.5 USDC)
    rates: HashMap<String, f64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 [Init]: تشغيل 3 Workers متوازية باقتناص مستمر للأسعار والبحث عن الفرص العكسية...\n");

    let client = Client::builder()
        .timeout(Duration::from_secs(4))
        .build()?;

    // ذاكرة مشتركة محمية وقابلة للقراءة/الكتابة بين جميع الخيوط
    let market_state = Arc::new(RwLock::new(MarketData::default()));

    // إعداد أزواج التداول (Direct & Reverse)
    let weth = "0x4200000000000000000000000000000000000006";
    let usdc = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
    let wbtc = "0x0555E30da8f98308EdB960aa94C0Db47230d2B9c";

    // 🧵 Worker 1: WETH -> USDC (Direct)
    let client1 = client.clone();
    let state1 = Arc::clone(&market_state);
    let task1 = tokio::spawn(async move {
        fetch_tight_loop(client1, state1, 1, weth, usdc, "1000000000000000000", "WETH_USDC", 1e18, 1e6).await;
    });

    // 🧵 Worker 2: USDC -> WETH (Reverse)
    let client2 = client.clone();
    let state2 = Arc::clone(&market_state);
    let task2 = tokio::spawn(async move {
        fetch_tight_loop(client2, state2, 2, usdc, weth, "2000000000", "USDC_WETH", 1e6, 1e18).await;
    });

    // 🧵 Worker 3: WETH -> WBTC (Alternative Route)
    let client3 = client.clone();
    let state3 = Arc::clone(&market_state);
    let task3 = tokio::spawn(async move {
        fetch_tight_loop(client3, state3, 3, weth, wbtc, "1000000000000000000", "WETH_WBTC", 1e18, 1e8).await;
    });

    // انتظار تشغيل المهام المتوازية معاً
    let _ = tokio::join!(task1, task2, task3);

    Ok(())
}

/// دالة تنفيذ الطلبات اللحظية المتتالية (Tight Loop) بدون أي انتظار زمني
async fn fetch_tight_loop(
    client: Client,
    state: Arc<RwLock<MarketData>>,
    worker_id: u8,
    sell_token: &'static str,
    buy_token: &'static str,
    amount: &'static str,
    pair_name: &'static str,
    sell_decimals: f64,
    buy_decimals: f64,
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
                            let buy_amount_str = json["buyAmount"].as_str().unwrap_or("0");
                            let buy_amount = buy_amount_str.parse::<f64>().unwrap_or(0.0);
                            let sell_amount = amount.parse::<f64>().unwrap_or(1.0);

                            // حساب سعر الوحدة الحقيقي
                            let rate = (buy_amount / buy_decimals) / (sell_amount / sell_decimals);

                            println!(
                                "⚡ [Worker {} | #{:04}]: {} | Latency: {:.1} ms | Rate: {:.6}",
                                worker_id, request_counter, pair_name, latency, rate
                            );

                            // 1. تحديث السعر في الذاكرة المشتركة
                            {
                                let mut lock = state.write().await;
                                lock.rates.insert(pair_name.to_string(), rate);
                            }

                            // 2. فحص وجود فرصة عكسية فوراً (Arbitrage Detection)
                            check_reverse_arbitrage(&state, pair_name).await;
                        }
                    }
                } else if status.as_u16() == 429 {
                    println!("⚠️ [Worker {}]: Rate Limit 429! نوم مؤقت 500ms...", worker_id);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
            Err(e) => {
                println!("❌ [Worker {}]: خطأ شبكة: {}", worker_id, e);
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }
}

/// دالة حساب ومقارنة الفرص العكسية بين الأوراق المالية في الذاكرة
async fn check_reverse_arbitrage(state: &Arc<RwLock<MarketData>>, updated_pair: &str) {
    let lock = state.read().await;

    // فحص الفرصة العكسية المباشرة بين WETH و USDC
    if updated_pair == "WETH_USDC" || updated_pair == "USDC_WETH" {
        if let (Some(&weth_to_usdc), Some(&usdc_to_weth)) = (
            lock.rates.get("WETH_USDC"),
            lock.rates.get("USDC_WETH"),
        ) {
            // المعامل العكسي: ضرب نسبة الذهاب في نسبة العودة
            let return_factor = weth_to_usdc * usdc_to_weth;
            let profit_percentage = (return_factor - 1.0) * 100.0;

            if return_factor > 1.0 {
                println!("\n🎯🎯🎯 [OPPORTUNITY DETECTED! / فرصة مراجحة عكسية] 🎯🎯🎯");
                println!("   ▶ Direct (WETH -> USDC): {:.4}", weth_to_usdc);
                println!("   ◀ Reverse (USDC -> WETH): {:.6}", usdc_to_weth);
                println!("   💰 Return Factor: {:.6} | Net Profit: +{:.3}%\n", return_factor, profit_percentage);
            }
        }
    }
}
