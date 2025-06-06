use worker::*;
use serde::{Deserialize, Serialize};
use chrono::{Utc, NaiveDate, Datelike};
use anyhow::{Result, anyhow};

/// Taiga 任务记录
#[derive(Debug, Serialize, Deserialize)]
pub struct TaigaTaskRecord {
    pub task_key: String,        // 格式：project-name#task_id
    pub first_seen_date: String, // 首次出现日期 YYYY-MM-DD
    pub last_seen_date: String,  // 最后出现日期 YYYY-MM-DD
    pub total_days: i32,         // 累积工作天数
}

/// 数据库操作客户端
pub struct DatabaseClient<'a> {
    db: &'a D1Database,
}

impl<'a> DatabaseClient<'a> {
    /// 创建新的数据库客户端
    pub fn new(db: &'a D1Database) -> Self {
        Self { db }
    }

    /// 初始化数据库表
    pub async fn init_tables(&self) -> Result<()> {
        let create_table_sql = r#"
            CREATE TABLE IF NOT EXISTS taiga_tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_key TEXT UNIQUE NOT NULL,
                first_seen_date TEXT NOT NULL,
                last_seen_date TEXT NOT NULL,
                total_days INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#;

        self.db.prepare(create_table_sql).run().await
            .map_err(|e| anyhow!("创建表失败: {:?}", e))?;

        Ok(())
    }

    /// 从 Taiga URL 中提取任务键
    /// 例如: https://tree.taiga.io/project/zenai-international-soraka/task/41 
    /// 提取为: zenai-international-soraka#41
    pub fn extract_task_key_from_url(url: &str) -> Option<String> {
        // 匹配 Taiga URL 模式
        if let Some(project_start) = url.find("/project/") {
            if let Some(task_start) = url.find("/task/") {
                let project_part = &url[project_start + 9..task_start]; // 跳过 "/project/"
                let task_part = &url[task_start + 6..]; // 跳过 "/task/"
                
                // 只取数字部分作为任务ID
                if let Some(task_id) = task_part.split('/').next() {
                    if task_id.chars().all(|c| c.is_ascii_digit()) {
                        return Some(format!("{}#{}", project_part, task_id));
                    }
                }
            }
        }
        None
    }

    /// 记录或更新 Taiga 任务
    pub async fn record_taiga_task(&self, task_key: &str) -> Result<i32> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        
        // 首先尝试获取现有记录
        if let Ok(existing_record) = self.get_task_record(task_key).await {
            // 如果任务已存在，更新最后出现日期并计算天数
            let total_days = self.calculate_work_days(&existing_record.first_seen_date, &today)?;
            
            let update_sql = r#"
                UPDATE taiga_tasks 
                SET last_seen_date = ?1, total_days = ?2, updated_at = CURRENT_TIMESTAMP
                WHERE task_key = ?3
            "#;
            
            self.db.prepare(update_sql)
                .bind(&[today.into(), total_days.into(), task_key.into()])?
                .run().await
                .map_err(|e| anyhow!("更新任务记录失败: {:?}", e))?;
            
            Ok(total_days)
        } else {
            // 如果任务不存在，创建新记录
            let insert_sql = r#"
                INSERT INTO taiga_tasks (task_key, first_seen_date, last_seen_date, total_days)
                VALUES (?1, ?2, ?3, 1)
            "#;
            
            self.db.prepare(insert_sql)
                .bind(&[task_key.into(), today.clone().into(), today.into()])?
                .run().await
                .map_err(|e| anyhow!("插入任务记录失败: {:?}", e))?;
            
            Ok(1)
        }
    }

    /// 获取任务记录
    pub async fn get_task_record(&self, task_key: &str) -> Result<TaigaTaskRecord> {
        let select_sql = r#"
            SELECT task_key, first_seen_date, last_seen_date, total_days
            FROM taiga_tasks 
            WHERE task_key = ?1
        "#;

        let result = self.db.prepare(select_sql)
            .bind(&[task_key.into()])?
            .first::<TaigaTaskRecord>(None).await
            .map_err(|e| anyhow!("查询任务记录失败: {:?}", e))?;

        result.ok_or_else(|| anyhow!("未找到任务记录"))
    }

    /// 获取任务的工作天数
    pub async fn get_task_days(&self, task_key: &str) -> Result<i32> {
        match self.get_task_record(task_key).await {
            Ok(record) => Ok(record.total_days),
            Err(_) => Ok(1), // 如果没有记录，默认为第一天
        }
    }

    /// 计算工作天数（排除周末）
    fn calculate_work_days(&self, start_date: &str, end_date: &str) -> Result<i32> {
        let start = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
            .map_err(|e| anyhow!("起始日期格式错误: {}", e))?;
        let end = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
            .map_err(|e| anyhow!("结束日期格式错误: {}", e))?;

        if end < start {
            return Ok(1);
        }

        let mut current = start;
        let mut work_days = 0;

        while current <= end {
            // 检查是否为工作日（周一到周五）
            let weekday = current.weekday();
            if weekday.number_from_monday() <= 5 {
                work_days += 1;
            }
            
            // 防止无限循环
            if current == end {
                break;
            }
            
            // 使用 chrono::Duration 来增加天数
            current = current + chrono::Duration::days(1);
        }

        Ok(work_days.max(1)) // 至少返回1天
    }

    /// 批量处理 Taiga URLs
    pub async fn process_taiga_urls(&self, urls: Vec<&str>) -> Result<Vec<(String, i32)>> {
        let mut results = Vec::new();
        
        for url in urls {
            if let Some(task_key) = Self::extract_task_key_from_url(url) {
                match self.record_taiga_task(&task_key).await {
                    Ok(days) => results.push((task_key, days)),
                    Err(e) => {
                        console_log!("处理 Taiga 任务 {} 失败: {}", task_key, e);
                        results.push((task_key, 1)); // 失败时默认为1天
                    }
                }
            }
        }
        
        Ok(results)
    }

    /// 清理旧的任务记录（超过30天未出现的任务）
    pub async fn cleanup_old_tasks(&self) -> Result<()> {
        let cleanup_date = (Utc::now().date_naive() - chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();

        let delete_sql = r#"
            DELETE FROM taiga_tasks 
            WHERE last_seen_date < ?1
        "#;

        self.db.prepare(delete_sql)
            .bind(&[cleanup_date.into()])?
            .run().await
            .map_err(|e| anyhow!("清理旧任务记录失败: {:?}", e))?;

        Ok(())
    }
} 