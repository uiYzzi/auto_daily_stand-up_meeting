use async_openai::{
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client, config::OpenAIConfig,
};
use anyhow::{Result, anyhow};
use std::env;
use futures::StreamExt;
use std::io::{self, Write};

/// AI 客户端，用于与 OpenAI 兼容的 API 通信
pub struct AIClient {
    client: Client<OpenAIConfig>,
}

impl AIClient {
    /// 创建新的 AI 客户端
    pub fn new() -> Result<Self> {
        // 从环境变量读取配置
        let api_key = env::var("OPENAI_API_KEY")
            .or_else(|_| env::var("AI_API_KEY"))
            .map_err(|_| anyhow!("未设置 OPENAI_API_KEY 或 AI_API_KEY 环境变量"))?;

        let base_url = env::var("OPENAI_BASE_URL")
            .or_else(|_| env::var("AI_BASE_URL"))
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        println!("AI 配置信息:");
        println!("- Base URL: {}", base_url);
        println!("- API Key: {}...{}", 
            &api_key.chars().take(8).collect::<String>(),
            &api_key.chars().rev().take(4).collect::<String>().chars().rev().collect::<String>()
        );

        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);

        let client = Client::with_config(config);

        Ok(Self { client })
    }

    /// 生成每日站会报告（流式输出）
    pub async fn generate_standup_report_stream(&self, prompt: &str) -> Result<()> {
        let system_message = ChatCompletionRequestSystemMessageArgs::default()
            .content("你是一个专业的项目管理助手，专门帮助生成每日站会报告。请严格按照用户提供的格式要求，基于 GitHub PR 数据生成简洁、专业的站会内容，并且不要输出多余的内容。")
            .build()?;

        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content(prompt)
            .build()?;

        let messages = vec![
            ChatCompletionRequestMessage::System(system_message),
            ChatCompletionRequestMessage::User(user_message),
        ];

        let model = env::var("OPENAI_MODEL")
            .or_else(|_| env::var("AI_MODEL"))
            .unwrap_or_else(|_| "gpt-3.5-turbo".to_string());

        let request = CreateChatCompletionRequestArgs::default()
            .model(&model)
            .messages(messages)
            .max_tokens(1000u16)
            .temperature(0.3)
            .stream(true)  // 启用流式输出
            .build()?;

        println!("正在调用 AI 生成站会报告...");
        println!("使用模型: {}", model);
        println!("🤖 AI 生成的每日站会报告：");
        println!("======================================");

        // 创建流式请求
        let mut stream = self.client.chat().create_stream(request).await?;

        // 处理流式响应
        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        if let Some(delta) = &choice.delta.content {
                            print!("{}", delta);
                            io::stdout().flush().unwrap(); // 确保立即输出
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\n❌ 流式响应错误: {}", e);
                    return Err(anyhow!("流式响应处理失败: {}", e));
                }
            }
        }

        println!();
        println!("======================================");
        println!();
        println!("💡 提示：您可以直接复制上述内容到飞书汇报功能中");

        Ok(())
    }
}

impl Default for AIClient {
    fn default() -> Self {
        Self::new().expect("无法创建 AI 客户端")
    }
}