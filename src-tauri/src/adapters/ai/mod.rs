pub mod cloud_asr;
pub mod cloud_rewriter;
pub mod local_asr;
pub mod local_rewriter;
mod qwen_client;

#[cfg(test)]
mod evaluation;

pub async fn test_qwen_rewrite_connection(api_key: String, model: String) -> Result<(), String> {
    use serde_json::json;

    let client = qwen_client::QwenClient::new(api_key)?;
    client
        .completion(json!({
            "model": model,
            "messages": [{ "role": "user", "content": "只回复 OK" }],
            "enable_thinking": false,
            "stream": false,
            "temperature": 0,
            "max_tokens": 4
        }))
        .await
        .map(|_| ())
}
