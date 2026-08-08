use std::time::Duration;

use futures_util::StreamExt;
use reqwest::{Client, Response, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use zeroize::Zeroize;

const CHAT_COMPLETIONS_URL: &str =
    "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions";
const RETRY_DELAY: Duration = Duration::from_millis(250);

pub struct QwenClient {
    client: Client,
    api_key: String,
    cancellation: CancellationToken,
}

impl Drop for QwenClient {
    fn drop(&mut self) {
        self.api_key.zeroize();
    }
}

impl QwenClient {
    pub fn new(api_key: String) -> Result<Self, String> {
        Self::with_cancellation(api_key, CancellationToken::new())
    }

    pub fn with_cancellation(
        api_key: String,
        cancellation: CancellationToken,
    ) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err("Qwen API key is empty".to_owned());
        }

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| format!("failed to create Qwen HTTP client: {error}"))?;
        Ok(Self {
            client,
            api_key,
            cancellation,
        })
    }

    pub fn from_env() -> Result<Self, String> {
        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .map_err(|_| "DASHSCOPE_API_KEY is not configured".to_owned())?;
        Self::new(api_key)
    }

    pub async fn completion(&self, payload: Value) -> Result<String, String> {
        let response = self.send(payload).await?;
        let completion = tokio::select! {
            _ = self.cancellation.cancelled() => return Err("Qwen request was cancelled".to_owned()),
            result = response.json::<CompletionResponse>() => result
                .map_err(|error| format!("Qwen response could not be decoded: {error}"))?,
        };

        completion
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| "Qwen returned an empty response".to_owned())
    }

    pub async fn streaming_completion(&self, payload: Value) -> Result<String, String> {
        let response = self.send(payload).await?;
        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut output = String::new();

        loop {
            let next = tokio::select! {
                _ = self.cancellation.cancelled() => return Err("Qwen request was cancelled".to_owned()),
                next = stream.next() => next,
            };
            let Some(chunk) = next else { break };
            let chunk = chunk.map_err(|error| format!("Qwen stream failed: {error}"))?;
            buffer.extend_from_slice(&chunk);

            while let Some(newline) = buffer.iter().position(|byte| *byte == b'\n') {
                let line = buffer.drain(..=newline).collect::<Vec<_>>();
                if parse_sse_line(&line, &mut output)? {
                    return finish_stream(output);
                }
            }
        }

        if !buffer.is_empty() {
            let _ = parse_sse_line(&buffer, &mut output)?;
        }
        finish_stream(output)
    }

    async fn send(&self, payload: Value) -> Result<Response, String> {
        for attempt in 0..=1 {
            let request = self
                .client
                .post(CHAT_COMPLETIONS_URL)
                .bearer_auth(&self.api_key)
                .json(&payload);
            let response = tokio::select! {
                _ = self.cancellation.cancelled() => return Err("Qwen request was cancelled".to_owned()),
                result = request.send() => result
                    .map_err(|error| format!("Qwen network request failed: {error}"))?,
            };
            let status = response.status();
            if status.is_success() {
                return Ok(response);
            }
            if attempt == 0 && is_retryable(status) {
                tokio::select! {
                    _ = self.cancellation.cancelled() => return Err("Qwen request was cancelled".to_owned()),
                    _ = tokio::time::sleep(RETRY_DELAY) => {}
                }
                continue;
            }
            return Err(sanitized_http_error(status));
        }
        Err("Qwen service request failed after retry".to_owned())
    }
}

fn parse_sse_line(line: &[u8], output: &mut String) -> Result<bool, String> {
    let line = std::str::from_utf8(line)
        .map_err(|_| "Qwen stream contained invalid UTF-8".to_owned())?
        .trim();
    let Some(data) = line.strip_prefix("data:") else {
        return Ok(false);
    };
    let data = data.trim();
    if data == "[DONE]" {
        return Ok(true);
    }
    if data.is_empty() {
        return Ok(false);
    }

    let chunk: StreamCompletionResponse = serde_json::from_str(data)
        .map_err(|_| "Qwen stream chunk could not be decoded".to_owned())?;
    for choice in chunk.choices {
        if let Some(content) = choice.delta.content {
            output.push_str(&content);
        }
    }
    Ok(false)
}

fn finish_stream(output: String) -> Result<String, String> {
    if output.trim().is_empty() {
        Err("Qwen returned an empty response".to_owned())
    } else {
        Ok(output.trim().to_owned())
    }
}

fn is_retryable(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT
    )
}

fn sanitized_http_error(status: StatusCode) -> String {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            "Qwen authentication failed; check the configured API key".to_owned()
        }
        StatusCode::TOO_MANY_REQUESTS => "Qwen rate limit or account quota was exceeded".to_owned(),
        _ => format!("Qwen service returned HTTP status {status}"),
    }
}

#[derive(Deserialize)]
struct CompletionResponse {
    choices: Vec<CompletionChoice>,
}

#[derive(Deserialize)]
struct CompletionChoice {
    message: CompletionMessage,
}

#[derive(Deserialize)]
struct CompletionMessage {
    content: String,
}

#[derive(Deserialize)]
struct StreamCompletionResponse {
    #[serde(default)]
    choices: Vec<StreamCompletionChoice>,
}

#[derive(Deserialize)]
struct StreamCompletionChoice {
    delta: StreamDelta,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_api_key() {
        assert!(QwenClient::new("  ".to_owned()).is_err());
    }

    #[test]
    fn authentication_error_does_not_include_secret_data() {
        assert_eq!(
            sanitized_http_error(StatusCode::UNAUTHORIZED),
            "Qwen authentication failed; check the configured API key"
        );
    }

    #[test]
    fn parses_streaming_delta_and_done_marker() {
        let mut output = String::new();
        assert!(!parse_sse_line(
            "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n".as_bytes(),
            &mut output,
        )
        .expect("parse delta"));
        assert!(parse_sse_line(b"data: [DONE]\n", &mut output).expect("parse done"));
        assert_eq!(output, "你");
    }
}
