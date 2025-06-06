-- 自动每日站会报告 - 数据库表结构
-- 用于记录 Taiga 任务的累积工作天数

-- Taiga 任务记录表
CREATE TABLE IF NOT EXISTS taiga_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_key TEXT UNIQUE NOT NULL,          -- 任务键，格式：project-name#task_id
    first_seen_date TEXT NOT NULL,         -- 首次出现日期 YYYY-MM-DD
    last_seen_date TEXT NOT NULL,          -- 最后出现日期 YYYY-MM-DD
    total_days INTEGER NOT NULL DEFAULT 1, -- 累积工作天数（排除周末）
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 索引优化
CREATE INDEX IF NOT EXISTS idx_taiga_tasks_key ON taiga_tasks(task_key);
CREATE INDEX IF NOT EXISTS idx_taiga_tasks_last_seen ON taiga_tasks(last_seen_date);

-- 插入示例数据（可选）
-- INSERT INTO taiga_tasks (task_key, first_seen_date, last_seen_date, total_days) 
-- VALUES ('zenai-international-soraka#41', '2024-01-15', '2024-01-15', 1); 