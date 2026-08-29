use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct ChatRequest {
    pub provider_id: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct ChatResponse {
    pub content: String,
    pub elapsed_ms: u128,
}

pub async fn complete(
    request: ChatRequest,
    base_url: &str,
    api_key: &str,
) -> Result<ChatResponse, String> {
    let started = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(600))
        .build().map_err(|e| e.to_string())?;
    let response=client.post(format!("{}/chat/completions",base_url.trim_end_matches('/'))).bearer_auth(api_key).json(&serde_json::json!({"model":request.model,"messages":request.messages,"temperature":0.2,"max_tokens":2048,"stream":false})).send().await.map_err(|e|e.to_string())?;
    let status = response.status();
    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(body
            .get("detail")
            .or_else(|| body.get("error"))
            .map(|v| v.to_string())
            .unwrap_or_else(|| format!("HTTP {status}")));
    }
    let content = body.get("choices").and_then(|v| v.get(0)).and_then(|v| v.get("message")).and_then(|v| v.get("content"))
        .and_then(|v| v.as_str()).unwrap_or("").to_string();
    if content.is_empty() { return Err("模型返回了空内容，请检查模型 ID 或接口兼容性".into()); }
    Ok(ChatResponse {
        content,
        elapsed_ms: started.elapsed().as_millis(),
    })
}
