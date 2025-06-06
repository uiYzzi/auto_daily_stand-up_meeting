use worker::*;
use serde_json;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

mod github_api;
mod ai_client;
mod feishu_webhook;

use github_api::GitHubApiClient;
use ai_client::AIClient;
use feishu_webhook::FeishuWebhook;

#[derive(Serialize, Deserialize)]
struct HolidayResponse {
    date: String,
    year: i32,
    month: i32,
    day: i32,
    status: i32,
}

#[event(start)]
fn start() {
    console_error_panic_hook::set_once();
}

#[event(scheduled)]
async fn scheduled_handler(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_log!("定时任务触发：检查今日是否为中国法定工作日");
    
    // 先检查今天是否为工作日
    match is_working_day().await {
        Ok(true) => {
            console_log!("✓ 今日为工作日，开始执行每日站会报告生成");
            match generate_and_send_daily_standup(&env).await {
                Ok(_) => {
                    console_log!("✓ 每日站会报告生成并发送成功");
                }
                Err(e) => {
                    console_log!("❌ 每日站会报告生成失败: {}", e.to_string());
                }
            }
        }
        Ok(false) => {
            console_log!("ℹ️ 今日为非工作日（周末或法定节假日），跳过站会报告生成");
        }
        Err(e) => {
            console_log!("⚠️ 检查工作日状态失败: {}，默认执行站会报告生成", e.to_string());
            match generate_and_send_daily_standup(&env).await {
                Ok(_) => {
                    console_log!("✓ 每日站会报告生成并发送成功");
                }
                Err(e) => {
                    console_log!("❌ 每日站会报告生成失败: {}", e.to_string());
                }
            }
        }
    }
}

#[event(fetch)]
async fn fetch_handler(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let url = req.url()?;
    
    match url.path() {
        "/health" => {
            Response::ok("服务运行正常")
        }
        "/manual-trigger" => {
            // 手动触发站会报告生成
            match generate_and_send_daily_standup(&env).await {
                Ok(report) => {
                    let response = serde_json::json!({
                        "success": true,
                        "message": "每日站会报告生成并发送成功",
                        "report": report
                    });
                    Response::from_json(&response)
                }
                Err(e) => {
                    let response = serde_json::json!({
                        "success": false,
                        "error": e.to_string()
                    });
                    Ok(Response::from_json(&response)?.with_status(500))
                }
            }
        }
        "/check-working-day" => {
            // 检查今天是否为工作日
            match is_working_day().await {
                Ok(is_working) => {
                    let response = serde_json::json!({
                        "success": true,
                        "is_working_day": is_working,
                        "message": if is_working { "今日为工作日" } else { "今日为非工作日" }
                    });
                    Response::from_json(&response)
                }
                Err(e) => {
                    let response = serde_json::json!({
                        "success": false,
                        "error": e.to_string()
                    });
                    Ok(Response::from_json(&response)?.with_status(500))
                }
            }
        }
        _ => {
            Response::error("Not found", 404)
        }
    }
}

async fn generate_and_send_daily_standup(env: &Env) -> Result<String> {
    // 获取环境变量
    let github_token = env.var("GITHUB_TOKEN")?.to_string();
    let openai_api_key = env.var("OPENAI_API_KEY")?.to_string();
    let openai_base_url = env.var("OPENAI_BASE_URL").map(|s| s.to_string()).unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let openai_model = env.var("OPENAI_MODEL").map(|s| s.to_string()).unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
    let feishu_webhook_url = env.var("FEISHU_WEBHOOK_URL")?.to_string();

    if github_token.is_empty() {
        return Err(Error::RustError("GITHUB_TOKEN 环境变量未设置".into()));
    }

    if feishu_webhook_url.is_empty() {
        return Err(Error::RustError("FEISHU_WEBHOOK_URL 环境变量未设置".into()));
    }

    console_log!("开始获取今日 GitHub PR 数据...");

    // 创建 GitHub API 客户端
    let github_client = GitHubApiClient::new(github_token);

    // 获取今天的 PR
    let pr_response = github_client.get_today_prs().await
        .map_err(|e| Error::RustError(format!("获取 GitHub PR 失败: {}", e)))?;

    console_log!("✓ 成功获取 {} 个 PR", pr_response.total_count);

    // 生成站会报告数据
    let standup_data = github_client.generate_standup_report(&pr_response);

    let final_report = if !openai_api_key.is_empty() {
        console_log!("正在使用 AI 生成格式化的站会报告...");
        
        // 创建 AI 客户端
        let ai_client = AIClient::new(openai_api_key, openai_base_url, openai_model);
        
        // 使用 AI 生成最终报告
        match ai_client.generate_standup_report(&standup_data).await {
            Ok(report) => {
                console_log!("✓ AI 报告生成成功");
                report
            }
            Err(e) => {
                console_log!("⚠️ AI 生成失败，使用原始数据: {}", e);
                standup_data
            }
        }
    } else {
        console_log!("⚠️ 未配置 OpenAI API，使用原始数据");
        standup_data
    };

    // 发送到飞书
    console_log!("正在发送报告到飞书...");
    let feishu_webhook = FeishuWebhook::new(feishu_webhook_url);
    
    feishu_webhook.send_standup_report(&final_report).await
        .map_err(|e| Error::RustError(format!("飞书消息发送失败: {}", e)))?;

    console_log!("✓ 报告已成功发送到飞书");

    Ok(final_report)
}

/// 检查今天是否为中国法定工作日
/// 返回 true 表示工作日（status = 0 或 2），false 表示非工作日（status = 1 或 3）
async fn is_working_day() -> Result<bool> {
    // 获取当前 UTC 时间
    let now = js_sys::Date::new_0();
    
    // 手动计算北京时间（UTC+8）
    let utc_timestamp = now.get_time(); // 毫秒时间戳
    let beijing_timestamp = utc_timestamp + (8.0 * 60.0 * 60.0 * 1000.0); // 加8小时
    let beijing_time = js_sys::Date::new(&JsValue::from_f64(beijing_timestamp));
    
    let year = beijing_time.get_full_year() as i32;
    let month = (beijing_time.get_month() + 1) as i32; // JavaScript月份从0开始
    let day = beijing_time.get_date() as i32;
    
    let date_str = format!("{:04}-{:02}-{:02}", year, month, day);
    let api_url = format!("http://api.haoshenqi.top/holiday?date={}", date_str);
    
    console_log!("正在查询日期 {} 的工作日状态...", date_str);
    
    // 发起HTTP请求
    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    
    let request = Request::new_with_init(&api_url, &init)?;
    let mut response = Fetch::Request(request).send().await?;
    
    if response.status_code() != 200 {
        return Err(Error::RustError(format!("节假日API请求失败，状态码: {}", response.status_code())));
    }
    
    // 先获取原始文本响应进行调试
    let response_text = response.text().await?;
    console_log!("节假日API原始响应: {}", response_text);
    
    // 尝试解析 JSON 数组
    let holiday_responses: Vec<HolidayResponse> = serde_json::from_str(&response_text)
        .map_err(|e| Error::RustError(format!("解析节假日API响应失败: {}", e)))?;
    
    // 获取第一个（也是唯一的）响应
    let holiday_response = holiday_responses.into_iter().next()
        .ok_or_else(|| Error::RustError("节假日API响应为空数组".into()))?;
    
    console_log!("节假日API响应: 日期={}, 状态={}", holiday_response.date, holiday_response.status);
    
    // status: 0普通工作日, 1周末双休日, 2需要补班的工作日, 3法定节假日
    // 只有 0 和 2 才是工作日
    match holiday_response.status {
        0 => {
            console_log!("✓ 今日为普通工作日");
            Ok(true)
        }
        1 => {
            console_log!("ℹ️ 今日为周末双休日");
            Ok(false)
        }
        2 => {
            console_log!("✓ 今日为需要补班的工作日");
            Ok(true)
        }
        3 => {
            console_log!("ℹ️ 今日为法定节假日");
            Ok(false)
        }
        _ => {
            console_log!("⚠️ 未知的工作日状态: {}", holiday_response.status);
            Ok(false)
        }
    }
} 