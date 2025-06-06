use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use worker::*;

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

/// AI 客户端，用于与 OpenAI 兼容的 API 通信
pub struct AIClient {
    api_key: String,
    base_url: String,
    model: String,
}

impl AIClient {
    /// 创建新的 AI 客户端
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
        }
    }

    /// 生成每日站会报告
    pub async fn generate_standup_report(&self, prompt: &str) -> Result<String> {
        let system_message = ChatMessage {
            role: "system".to_string(),
            content: "你是一个专业的项目管理助手，专门帮助生成每日站会报告。请严格按照用户提供的格式要求，基于 GitHub PR 数据生成简洁、专业的站会内容，并且不要输出多余的内容。".to_string(),
        };

        let user_message = ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        };

        let request_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![system_message, user_message],
            max_tokens: 1000,
            temperature: 0.3,
        };

        let url = format!("{}/chat/completions", self.base_url);

        // 创建请求头
        let mut headers = worker::Headers::new();
        headers.set("Content-Type", "application/json")?;
        headers.set("Authorization", &format!("Bearer {}", self.api_key))?;

        let mut request_init = RequestInit::new();
        request_init.method = Method::Post;
        request_init.headers = headers;
        request_init.body = Some(serde_json::to_string(&request_body)?.into());

        let request = Request::new_with_init(&url, &request_init)?;

        let mut response = Fetch::Request(request).send().await?;

        if !(200..300).contains(&response.status_code()) {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("AI API 请求失败: {} - {}", response.status_code(), error_text));
        }

        let completion_response: ChatCompletionResponse = response.json().await?;

        if let Some(choice) = completion_response.choices.first() {
            Ok(choice.message.content.trim().to_string())
        } else {
            Err(anyhow!("AI API 返回空响应"))
        }
    }
} 