use reqwest::Client;
use serde::{Deserialize, Serialize};
use chrono::{Local, NaiveDate};
use anyhow::{Result, anyhow};
use regex::Regex;

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
    pub labels_url: String,
    pub comments_url: String,
    pub events_url: String,
    pub html_url: String,
    pub id: u64,
    pub node_id: String,
    pub number: u32,
    pub title: String,
    pub user: User,
    pub labels: Vec<serde_json::Value>,
    pub state: String,
    pub locked: bool,
    pub assignee: Option<User>,
    pub assignees: Vec<User>,
    pub milestone: Option<serde_json::Value>,
    pub comments: u32,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub author_association: String,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub active_lock_reason: Option<String>,
    pub draft: bool,
    pub pull_request: PullRequestInfo,
    pub body: String,
    pub reactions: Reactions,
    pub timeline_url: String,
    pub performed_via_github_app: Option<serde_json::Value>,
    pub state_reason: Option<String>,
    pub score: f64,
}

/// 用户信息
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
    pub node_id: String,
    pub avatar_url: String,
    pub gravatar_id: String,
    pub url: String,
    pub html_url: String,
    pub followers_url: String,
    pub following_url: String,
    pub gists_url: String,
    pub starred_url: String,
    pub subscriptions_url: String,
    pub organizations_url: String,
    pub repos_url: String,
    pub events_url: String,
    pub received_events_url: String,
    #[serde(rename = "type")]
    pub user_type: String,
    pub user_view_type: Option<String>,
    pub site_admin: bool,
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

/// 反应信息
#[derive(Debug, Serialize, Deserialize)]
pub struct Reactions {
    pub url: String,
    pub total_count: u32,
    #[serde(rename = "+1")]
    pub plus_one: u32,
    #[serde(rename = "-1")]
    pub minus_one: u32,
    pub laugh: u32,
    pub hooray: u32,
    pub confused: u32,
    pub heart: u32,
    pub rocket: u32,
    pub eyes: u32,
}

/// GitHub API 客户端
pub struct GitHubApiClient {
    client: Client,
    token: Option<String>,
}

impl GitHubApiClient {
    /// 创建新的 GitHub API 客户端
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
        }
    }

    /// 设置 GitHub 访问令牌
    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    /// 获取当天创建的 PR
    pub async fn get_today_prs(&self) -> Result<GitHubSearchResponse> {
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
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

        let mut request_builder = self.client.get(&url)
            .header("User-Agent", "auto-daily-standup-meeting")
            .header("Accept", "application/vnd.github.v3+json");

        // 如果有 token，添加到请求头
        if let Some(ref token) = self.token {
            request_builder = request_builder.header("Authorization", format!("token {}", token));
        }

        let response = request_builder.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("GitHub API 请求失败: {} - {}", status, error_text));
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
                let taiga_info = self.extract_taiga_info(&pr.title, &pr.body);
                if !taiga_info.is_empty() {
                    report.push_str(&format!("- 关联 Taiga：{}\n", taiga_info));
                }
                
                // 从 Taiga 链接中提取项目代号，如果没有则从仓库名称中提取
                let project_code = if !taiga_info.is_empty() && taiga_info.starts_with("https://tree.taiga.io/") {
                    self.extract_project_code_from_taiga(&taiga_info)
                } else {
                    self.extract_project_code(&pr.repository_url)
                };
                if !project_code.is_empty() {
                    report.push_str(&format!("- 项目代号：{}\n", project_code));
                }
                
                // 添加 PR 描述的关键部分
                if !pr.body.is_empty() {
                    let summary = self.extract_work_summary(&pr.body);
                    if !summary.is_empty() {
                        report.push_str(&format!("- 工作内容摘要：{}\n", summary));
                    }
                }
                
                report.push_str(&format!("- 创建时间：{}\n", pr.created_at));
                report.push_str(&format!("- 链接：{}\n\n", pr.html_url));
            }
        }

        // 添加 AI 提示词
        report.push_str(&self.generate_ai_prompt());
        
        report
    }

    /// 从标题和描述中提取 Taiga issue 信息
    fn extract_taiga_info(&self, title: &str, body: &str) -> String {
        let content = format!("{} {}", title, body);
        
        // 查找 Taiga 链接模式
        if let Some(start) = content.find("https://tree.taiga.io/") {
            if let Some(end) = content[start..].find(char::is_whitespace) {
                return content[start..start + end].to_string();
            } else {
                // 如果没有找到空白字符，取到字符串末尾
                return content[start..].to_string();
            }
        }
        
        // 查找 #数字 模式
        let re = Regex::new(r"#(\d+)").unwrap();
        if let Some(captures) = re.captures(&content) {
            if let Some(issue_num) = captures.get(1) {
                return format!("#{}", issue_num.as_str());
            }
        }
        
        String::new()
    }

    /// 从仓库 URL 中提取项目代号
    fn extract_project_code(&self, repo_url: &str) -> String {
        // 从 https://api.github.com/repos/组织名/仓库名 中提取仓库名
        let cleaned_url = repo_url.replace("https://api.github.com/repos/", "");
        
        // 按 '/' 分割，取最后一部分作为仓库名
        cleaned_url
            .split('/')
            .last()
            .unwrap_or(&cleaned_url)
            .to_string()
    }

    /// 从 Taiga 链接中提取项目代号
    fn extract_project_code_from_taiga(&self, taiga_url: &str) -> String {
        if let Some(start) = taiga_url.find("https://tree.taiga.io/project/") {
            let after_project = &taiga_url[start + "https://tree.taiga.io/project/".len()..];
            if let Some(end) = after_project.find('/') {
                return after_project[..end].to_string();
            } else {
                // 如果没有找到结束的 '/'，返回剩余的全部内容
                return after_project.to_string();
            }
        }
        String::new()
    }

    /// 从 PR 描述中提取工作内容摘要
    fn extract_work_summary(&self, body: &str) -> String {
        // 查找常见的描述模式
        let lines: Vec<&str> = body.lines().collect();
        
        for line in &lines {
            if line.contains("简介") || line.contains("主要改动") || line.contains("变更内容") {
                // 找到下一行作为摘要
                if let Some(pos) = lines.iter().position(|&l| l == *line) {
                    if pos + 1 < lines.len() {
                        let summary = lines[pos + 1].trim();
                        if !summary.is_empty() && !summary.starts_with('#') && !summary.starts_with('-') {
                            return summary.chars().take(50).collect::<String>() + 
                                   if summary.len() > 50 { "..." } else { "" };
                        }
                    }
                }
            }
        }
        
        // 如果没有找到标准格式，取第一个非空行作为摘要
        for line in &lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() && 
               !trimmed.starts_with('#') && 
               !trimmed.starts_with("```") &&
               !trimmed.starts_with("http") &&
               trimmed.len() > 10 {
                return trimmed.chars().take(50).collect::<String>() + 
                       if trimmed.len() > 50 { "..." } else { "" };
            }
        }
        
        String::new()
    }

    /// 生成 AI 提示词
    fn generate_ai_prompt(&self) -> String {
        r#"
=== AI 提示词 ===

请根据上述 GitHub PR 数据，生成符合以下格式的每日站会报告：

每日站会格式要求

1. 今日完成工作
格式要求：
- 有对应 Taiga issue 的：[天数]项目代号#Taiga编号-工作内容
- 无对应 Taiga issue 的：[天数]工作内容
- [天数] 是指这项工作到目前为止累积的天数

示例：
[2]XXXAsk#3-重构登录逻辑
[1]KTVIIU#15-完成功能开发，准备测试
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

请基于上述数据生成今日的站会报告内容。如果没有足够的信息推断天数，可以标记为 [1] 表示新开始的工作。请输出纯文本，不要使用 markdown，为了避免涉密，请不要输出任何与项目相关的信息，尽量简要描述今天的工作内容就够了。
"#.to_string()
    }
}

impl Default for GitHubApiClient {
    fn default() -> Self {
        Self::new()
    }
} 