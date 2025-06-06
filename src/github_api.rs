use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, Utc};
use anyhow::{Result, anyhow};
use regex::Regex;
use worker::*;

/// GitHub API 响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubSearchResponse {
    pub total_count: u32,
    pub incomplete_results: bool,
    pub items: Vec<PullRequestItem>,
}

/// PR 项目信息
#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequestItem {
    pub url: String,
    pub repository_url: String,
    pub html_url: String,
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub user: User,
    pub state: String,
    pub draft: bool,
    pub pull_request: PullRequestInfo,
    pub body: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
}

/// 用户信息
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
    pub avatar_url: String,
    pub html_url: String,
}

/// PR 详细信息
#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub url: String,
    pub html_url: String,
    pub diff_url: String,
    pub patch_url: String,
    pub merged_at: Option<String>,
}

/// GitHub API 客户端
pub struct GitHubApiClient {
    token: String,
}

impl GitHubApiClient {
    /// 创建新的 GitHub API 客户端
    pub fn new(token: String) -> Self {
        Self { token }
    }

    /// 获取当天创建的 PR
    pub async fn get_today_prs(&self) -> Result<GitHubSearchResponse> {
        // 获取当前日期（UTC时间）
        let today = Utc::now().format("%Y-%m-%d").to_string();
        self.get_prs_by_date(&today).await
    }

    /// 获取指定日期创建的 PR
    pub async fn get_prs_by_date(&self, date: &str) -> Result<GitHubSearchResponse> {
        // 验证日期格式
        if NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            return Err(anyhow!("日期格式无效，请使用 YYYY-MM-DD 格式"));
        }

        let url = format!(
            "https://api.github.com/search/issues?q=is:pr+author:@me+created:{}",
            date
        );

        // 创建请求头
        let mut headers = worker::Headers::new();
        headers.set("User-Agent", "auto-daily-standup-worker")?;
        headers.set("Accept", "application/vnd.github.v3+json")?;
        headers.set("Authorization", &format!("token {}", self.token))?;

        let mut request_init = RequestInit::new();
        request_init.method = Method::Get;
        request_init.headers = headers;

        let request = Request::new_with_init(&url, &request_init)?;

        let mut response = Fetch::Request(request).send().await?;

        if !(200..300).contains(&response.status_code()) {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("GitHub API 请求失败: {} - {}", response.status_code(), error_text));
        }

        let search_response: GitHubSearchResponse = response.json().await?;
        Ok(search_response)
    }

    /// 生成每日站会报告格式
    pub fn generate_standup_report(&self, response: &GitHubSearchResponse) -> String {
        let mut report = String::new();
        
        report.push_str("=== 每日站会报告数据 ===\n\n");
        report.push_str("## 原始 GitHub PR 数据摘要\n");
        report.push_str(&format!("- 今日创建的 PR 总数：{}\n", response.total_count));
        report.push_str(&format!("- 数据完整性：{}\n\n", if response.incomplete_results { "部分数据" } else { "完整数据" }));

        if response.items.is_empty() {
            report.push_str("今天没有创建任何 PR，可能没有代码提交工作完成。\n\n");
        } else {
            report.push_str("## PR 详细信息\n\n");
            
            for (index, pr) in response.items.iter().enumerate() {
                report.push_str(&format!("### PR #{}\n", index + 1));
                report.push_str(&format!("- 标题：{}\n", pr.title));
                report.push_str(&format!("- 仓库：{}\n", 
                    pr.repository_url.replace("https://api.github.com/repos/", "")));
                report.push_str(&format!("- 状态：{}\n", 
                    if let Some(_) = &pr.pull_request.merged_at {
                        "已合并"
                    } else if pr.state == "closed" {
                        "已关闭"
                    } else {
                        "进行中"
                    }
                ));
                
                // 尝试从标题和描述中提取 Taiga issue 信息
                let body_content = pr.body.as_deref().unwrap_or("");
                let taiga_info = self.extract_taiga_info(&pr.title, body_content);
                if !taiga_info.is_empty() {
                    report.push_str(&format!("- 关联 Taiga：{}\n", taiga_info));
                }
                
                // 提取项目代号
                let project_code = self.extract_project_code(&pr.repository_url);
                if !project_code.is_empty() {
                    report.push_str(&format!("- 项目代号：{}\n", project_code));
                }
                
                // 提取工作总结
                let work_summary = self.extract_work_summary(body_content);
                if !work_summary.is_empty() {
                    report.push_str(&format!("- 工作内容：{}\n", work_summary));
                }
                
                report.push_str(&format!("- 链接：{}\n", pr.html_url));
                report.push('\n');
            }
        }

        // 添加 AI 处理提示
        report.push_str("## AI 处理指引\n");
        report.push_str(&self.generate_ai_prompt());
        
        report
    }

    /// 从标题和 PR 描述中提取 Taiga issue 信息
    fn extract_taiga_info(&self, title: &str, body: &str) -> String {
        let combined_text = format!("{} {}", title, body);
        
        // 匹配 Taiga URL 模式
        let taiga_url_regex = Regex::new(r"https://[^\s/]+\.taiga\.io/project/[^/]+/task/(\d+)").unwrap();
        if let Some(captures) = taiga_url_regex.captures(&combined_text) {
            if let Some(task_id) = captures.get(1) {
                return format!("Task #{}", task_id.as_str());
            }
        }
        
        // 匹配 #数字 模式
        let hash_number_regex = Regex::new(r"#(\d+)").unwrap();
        if let Some(captures) = hash_number_regex.captures(&combined_text) {
            if let Some(task_id) = captures.get(1) {
                return format!("Task #{}", task_id.as_str());
            }
        }
        
        String::new()
    }

    /// 从仓库 URL 中提取项目代号
    fn extract_project_code(&self, repo_url: &str) -> String {
        // 从 GitHub 仓库 URL 中取项目名称作为代号
        if let Some(repo_name) = repo_url.split('/').last() {
            return repo_name.to_uppercase();
        }
        String::new()
    }

    /// 从 PR 描述中提取工作总结
    fn extract_work_summary(&self, body: &str) -> String {
        if body.is_empty() {
            return String::new();
        }

        // 使用字符数量而不是字节数量来安全地截取字符串
        let summary = if body.chars().count() > 200 {
            let truncated: String = body.chars().take(200).collect();
            format!("{}...", truncated)
        } else {
            body.to_string()
        };

        // 清理换行和多余空格
        summary.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// 生成 AI 处理提示
    fn generate_ai_prompt(&self) -> String {
        r#"请基于上述 GitHub PR 数据，生成符合以下格式的每日站会报告：

格式要求：
- 有对应 Taiga issue 的：[天数]项目代号#Taiga编号-工作内容
- 无对应 Taiga issue 的：[天数]工作内容
- [天数] 是指这项工作到目前为止累积的天数

示例：
[2]xxxAsk#3-重构登录逻辑
[1]ktv#15-完成功能开发，准备测试
[3]学习Flutter和Dart

生成要求

1. 内容应简洁明了，避免冗长描述
2. 机密项目应避免透露敏感信息
3. 耗时一小时以下的工作无需汇报
4. 根据 PR 状态推断工作进度：
   - 已合并的 PR = 工作已完成
   - 进行中的 PR = 工作正在进行
   - 已关闭未合并的 PR = 工作可能取消或需要重新开始
5. 请自动提取项目代号，去除项目代号中无关部分，例如组织名、子模块名等等，例如 XXX-International-Corp/XXXX_flutter 提取为 XXXX，去除了XXX-International-Corp/和_flutter
6. 项目代号不要全部大写！

请基于上述数据生成今日的站会报告内容。如果没有足够的信息推断天数，可以标记为 [1] 表示新开始的工作。
请输出纯文本，不要使用 markdown，不要包含任何 markdown 语法。
为了避免涉密，请不要输出任何与项目相关的信息，尽量简要描述今天的工作内容就够了。"#.to_string()
    }
} 