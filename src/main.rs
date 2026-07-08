use reqwest::Client;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 [Init]: بدء إرسال الطلب إلى 0x API لقياس زمن الشبكة بدقة...");

    // 1. تهيئة عميل HTTP
    let client = Client::new();

    // 2. إعداد الرابط والمتغيرات
    let url = "https://api.0x.org/swap/allowance-holder/price";
    let params = [
        ("chainId", "8453"),
        ("sellToken", "0x4200000000000000000000000000000000000006"), // WETH
        ("buyToken", "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),  // USDC
        ("sellAmount", "1000000000000000000"), // 1 WETH
    ];
    let api_key = "33496247-f998-476f-bc8e-34779b69bd87";

    // 3. بدء العداد الزمني بدقة فائقة
    let start_time = Instant::now();

    // 4. إرسال الطلب وانتظار الرد
    let response = client.get(url)
        .query(&params)
        .header("0x-api-key", api_key)
        .header("0x-version", "v2")
        .send()
        .await?;

    // 5. استخراج حالة الرد والنص (يجب قراءة النص لضمان انتهاء النقل الشبكي)
    let status = response.status();
    let body = response.text().await?;

    // 6. إيقاف العداد الزمني
    let latency = start_time.elapsed();

    // 7. طباعة النتائج
    println!("✅ [Status]: {}", status);
    
    // عرض الزمن بالمللي ثانية والميكروثانية لترى الدقة المطلقة
    println!("⏱️ [Network Latency]: {} ms ({} µs)", latency.as_millis(), latency.as_micros());
    
    println!("📦 [Response Body Preview]:\n{:.300}...", body); // نطبع أول 300 حرف فقط لتجنب ازدحام الكونسول

    Ok(())
}
