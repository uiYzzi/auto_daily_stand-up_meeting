use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use worker::*;

#[derive(Serialize)]
struct FeishuMessage {
    msg_type: String,
    content: FeishuContent,
}

#[derive(Serialize)]
struct FeishuContent {
    text: String,
}

#[derive(Deserialize)]
struct FeishuResponse {
    code: i32,
    msg: String,
}

/// 飞书 Webhook 客户端
pub struct FeishuWebhook {
    webhook_url: String,
}

impl FeishuWebhook {
    /// 创建新的飞书 Webhook 客户端
    pub fn new(webhook_url: String) -> Self {
        Self { webhook_url }
    }

    /// 发送文本消息到飞书
    pub async fn send_message(&self, text: &str) -> Result<()> {
        let message = FeishuMessage {
            msg_type: "text".to_string(),
            content: FeishuContent {
                text: text.to_string(),
            },
        };

        // 创建请求头
        let mut headers = worker::Headers::new();
        headers.set("Content-Type", "application/json")?;

        let mut request_init = RequestInit::new();
        request_init.method = Method::Post;
        request_init.headers = headers;
        request_init.body = Some(serde_json::to_string(&message)?.into());

        let request = Request::new_with_init(&self.webhook_url, &request_init)?;

        let mut response = Fetch::Request(request).send().await?;

        if !(200..300).contains(&response.status_code()) {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "飞书 Webhook 请求失败: {} - {}",
                response.status_code(),
                error_text
            ));
        }

        // 检查飞书 API 响应
        let feishu_response: FeishuResponse = response.json().await?;
        
        if feishu_response.code != 0 {
            return Err(anyhow!(
                "飞书 API 返回错误: {} - {}",
                feishu_response.code,
                feishu_response.msg
            ));
        }

        Ok(())
    }

    /// 发送格式化的站会报告到飞书
    pub async fn send_standup_report(&self, report: &str) -> Result<()> {
        let formatted_message = format!(
            "📋 每日站会报告\n{}\n\n⏰ 生成时间: {}",
            report,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.send_message(&formatted_message).await
    }
} 