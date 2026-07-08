use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::interval;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 [Init]: بدء اختبار 0x API بمعدل 2 طلب/ثانية لمدة 10 دقائق (1,200 طلب)...");

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let url = "https://api.0x.org/swap/allowance-holder/price";
    let params = [
        ("chainId", "8453"),
        ("sellToken", "0x4200000000000000000000000000000000000006"), // WETH
        ("buyToken", "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),  // USDC
        ("sellAmount", "1000000000000000000"),                     // 1 WETH
    ];
    let api_key = "33496247-f998-476f-bc8e-34779b69bd87";

    // إعداد مؤقت ينطلق كل 500 مللي ثانية (2 طلب في الثانية)
    let mut ticker = interval(Duration::from_millis(500));

    // 2 طلب/ثانية * 600 ثانية = 1200 طلب
    let total_target_requests = 1200;

    let mut total_requests = 0;
    let mut success_count = 0;
    let mut rate_limit_count = 0;
    let mut error_count = 0;

    let mut latencies_ms: Vec<f64> = Vec::new();
    let start_total_time = Instant::now();

    while total_requests < total_target_requests {
        // الانتظار حتى يحين وقت الطلب القادم
        ticker.tick().await;

        total_requests += 1;
        let req_start = Instant::now();

        match client
            .get(url)
            .query(&params)
            .header("0x-api-key", api_key)
            .header("0x-version", "v2")
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                let latency = req_start.elapsed().as_secs_f64() * 1000.0;

                if status.is_success() {
                    if let Ok(body_text) = response.text().await {
                        latencies_ms.push(latency);
                        success_count += 1;

                        // استخراج السعر المرتجع لـ 1 WETH باستخدام Turbofish syntax ::<Value>
                        if let Ok(json) = serde_json::from_str::<Value>(&body_text) {
                            let buy_amount = json["buyAmount"].as_str().unwrap_or("0");
                            let buy_usdc = buy_amount.parse::<f64>().unwrap_or(0.0) / 1_000_000.0;

                            println!(
                                "req #{:04}/{}: ✅ 200 OK | Latency: {:.2} ms | 1 WETH = {:.2} USDC",
                                total_requests, total_target_requests, latency, buy_usdc
                            );
                        } else {
                            println!(
                                "req #{:04}/{}: ✅ 200 OK | Latency: {:.2} ms",
                                total_requests, total_target_requests, latency
                            );
                        }
                    }
                } else if status.as_u16() == 429 {
                    rate_limit_count += 1;
                    println!(
                        "req #{:04}/{}: ⚠️ 429 Rate Limit Exceeded | Latency: {:.2} ms",
                        total_requests, total_target_requests, latency
                    );
                } else {
                    error_count += 1;
                    println!(
                        "req #{:04}/{}: ❌ Status Error: {} | Latency: {:.2} ms",
                        total_requests, total_target_requests, status, latency
                    );
                }
            }
            Err(err) => {
                error_count += 1;
                println!(
                    "req #{:04}/{}: 💥 Network/Timeout Error: {}",
                    total_requests, total_target_requests, err
                );
            }
        }
    }

    // 📊 حساب الأحصائيات النهائية
    let elapsed_total = start_total_time.elapsed().as_secs_f64();
    let avg_latency = if !latencies_ms.is_empty() {
        latencies_ms.iter().sum::<f64>() / latencies_ms.len() as f64
    } else {
        0.0
    };

    let min_latency = latencies_ms.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_latency = latencies_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    println!("\n===========================================");
    println!("📊 [التقرير النهائي للتجربة - 10 دقائق]");
    println!("===========================================");
    println!("⏱️  الوقت الإجمالي للتجربة: {:.2} ثانية", elapsed_total);
    println!("📥  إجمالي الطلبات المرسلة: {}", total_requests);
    println!("✅  الطلبات الناجحة: {}", success_count);
    println!("⚠️  أخطاء تجاوز الحدود (429 Rate Limit): {}", rate_limit_count);
    println!("❌  أخطاء الاتصال/السيرفر: {}", error_count);
    println!("⚡  متوسط زمن الشبكة (Avg Latency): {:.2} ms", avg_latency);
    if !latencies_ms.is_empty() {
        println!("🏎️  أسرع طلب (Min Latency): {:.2} ms", min_latency);
        println!("🐢  أبطأ طلب (Max Latency): {:.2} ms", max_latency);
    }
    println!("===========================================\n");

    Ok(())
}
