# 数据库设置说明

本项目使用 Cloudflare D1 数据库来记录 Taiga 任务的累积工作天数，以便为 AI 生成更准确的站会报告。

## 1. 创建 D1 数据库

```bash
# 创建 D1 数据库
wrangler d1 create auto-daily-standup-db
```

执行后会返回数据库信息，类似：
```
✅ Successfully created DB 'auto-daily-standup-db'

[[d1_databases]]
binding = "DB"
database_name = "auto-daily-standup-db"
database_id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
```

## 2. 更新配置文件

将返回的 `database_id` 填入 `wrangler.toml` 文件中：

```toml
# D1 数据库绑定
[[d1_databases]]
binding = "DB"
database_name = "auto-daily-standup-db"
database_id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"  # 填入你的数据库ID
```

## 3. 初始化数据库表

使用提供的 schema.sql 文件初始化数据库表：

```bash
# 在本地环境初始化
wrangler d1 execute auto-daily-standup-db --local --file=./schema.sql

# 在生产环境初始化
wrangler d1 execute auto-daily-standup-db --remote --file=./schema.sql
```

## 4. 验证数据库设置

可以查询数据库确认表已创建：

```bash
# 查看表结构
wrangler d1 execute auto-daily-standup-db --command="SELECT name FROM sqlite_master WHERE type='table';"
```

## 5. 功能说明

### Taiga 任务记录
- 自动从 PR 描述中提取 Taiga URL
- 记录任务首次出现和最后出现的日期
- 计算累积工作天数（排除周末）
- 为 AI 生成站会报告提供天数信息

### 数据格式
- Taiga URL: `https://tree.taiga.io/project/zenai-international-soraka/task/41`
- 数据库记录: `zenai-international-soraka#41`
- 报告格式: `[2]项目#41-工作内容`

### 自动清理
- 系统会自动清理超过30天未出现的任务记录
- 保持数据库的整洁和性能

## 6. 常见问题

### Q: 如何查看现有的任务记录？
```bash
wrangler d1 execute auto-daily-standup-db --command="SELECT * FROM taiga_tasks ORDER BY updated_at DESC LIMIT 10;"
```

### Q: 如何手动添加任务记录？
```bash
wrangler d1 execute auto-daily-standup-db --command="INSERT INTO taiga_tasks (task_key, first_seen_date, last_seen_date, total_days) VALUES ('project-name#123', '2024-01-15', '2024-01-15', 1);"
```

### Q: 如何重置数据库？
```bash
wrangler d1 execute auto-daily-standup-db --command="DROP TABLE IF EXISTS taiga_tasks;"
wrangler d1 execute auto-daily-standup-db --file=./schema.sql
```

## 7. 部署注意事项

1. 先在本地测试数据库功能
2. 确保生产环境和本地环境都已初始化表结构
3. 部署后可通过 `/manual-trigger` 端点测试功能
4. 查看 Cloudflare Workers 日志确认数据库操作正常

完成这些步骤后，系统就能自动记录和追踪 Taiga 任务的工作天数了。 