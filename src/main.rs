mod api;
mod ai;

use api::GitHubApiClient;
use ai::AIClient;
use std::env;

#[tokio::main]
async fn main() {
    // 加载 .env 文件中的环境变量
    dotenv::dotenv().ok();
    
    println!("自动化每日站会报告生成器");
    println!("======================================");

    // 从环境变量获取 GitHub token（必需）
    let github_token = match env::var("GITHUB_TOKEN") {
        Ok(token) if !token.trim().is_empty() => {
            println!("✓ GitHub Token 已配置");
            token
        },
        _ => {
            eprintln!("❌ 错误：未设置 GITHUB_TOKEN 环境变量或令牌为空");
            eprintln!();
            eprintln!("请按以下步骤设置 GitHub Token：");
            eprintln!("方法 1 - 使用 .env 文件（推荐）：");
            eprintln!("1. 在项目根目录创建 .env 文件");
            eprintln!("2. 在文件中添加：GITHUB_TOKEN=your_token_here");
            eprintln!();
            eprintln!("方法 2 - 使用环境变量：");
            eprintln!("1. 访问 https://github.com/settings/tokens");
            eprintln!("2. 点击 'Generate new token (classic)'");
            eprintln!("3. 选择适当的权限（建议勾选 'repo' 或 'public_repo'）");
            eprintln!("4. 复制生成的 token");
            eprintln!("5. 在终端中设置环境变量：");
            eprintln!("   export GITHUB_TOKEN=your_token_here");
            eprintln!();
            eprintln!("然后重新运行程序。");
            std::process::exit(1);
        }
    };

    // 创建 GitHub API 客户端
    let github_client = GitHubApiClient::new().with_token(github_token);

    // 创建 AI 客户端
    let ai_client = match AIClient::new() {
        Ok(client) => {
            println!("✓ AI 客户端已配置");
            Some(client)
        },
        Err(e) => {
            eprintln!("⚠️  AI 客户端配置失败: {}", e);
            eprintln!();
            eprintln!("请在 .env 文件中添加 AI 配置（可选）：");
            eprintln!("OPENAI_API_KEY=your_openai_api_key");
            eprintln!("OPENAI_BASE_URL=https://api.openai.com/v1  # 可选，默认为 OpenAI");
            eprintln!("OPENAI_MODEL=gpt-3.5-turbo  # 可选，默认为 gpt-3.5-turbo");
            eprintln!();
            eprintln!("程序将继续运行，只输出原始数据...");
            None
        }
    };

    println!();
    println!("正在获取今日 PR 数据...");

    // 获取今天的 PR
    match github_client.get_today_prs().await {
        Ok(response) => {
            // 生成每日站会报告数据
            let standup_data = github_client.generate_standup_report(&response);
            
            println!("✓ 成功获取 {} 个 PR", response.total_count);
            println!();

            if let Some(ai_client) = ai_client {
                // 使用 AI 生成最终报告（流式输出）
                match ai_client.generate_standup_report_stream(&standup_data).await {
                    Ok(()) => {
                        // 流式输出已经在方法内部完成
                    },
                    Err(e) => {
                        eprintln!("❌ AI 生成报告失败: {}", e);
                        eprintln!();
                        eprintln!("原始数据输出：");
                        println!("{}", standup_data);
                    }
                }
            } else {
                // 如果没有 AI 客户端，只输出原始数据
                println!("📋 原始站会数据（请复制给 AI 助手处理）：");
                println!("======================================");
                println!("{}", standup_data);
                println!("======================================");
            }
        },
        Err(e) => {
            eprintln!("❌ 获取 PR 信息失败: {}", e);
            eprintln!("\n可能的解决方案:");
            eprintln!("1. 检查 GITHUB_TOKEN 是否有效");
            eprintln!("2. 确认 Token 具有足够的权限");
            eprintln!("3. 检查网络连接");
            eprintln!("4. 确认 GitHub API 可访问");
            std::process::exit(1);
        }
    }
}
