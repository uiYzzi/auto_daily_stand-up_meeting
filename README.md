# Cloudflare Workers 部署指南

这个项目已经适配为可以在 Cloudflare Workers 上运行的版本，每天北京时间下午 6 点自动生成每日站会报告并发送到飞书群聊。

## 功能特性

- 🕖 **智能定时执行**：每天北京时间下午 6 点自动检查，仅在中国法定工作日运行
- 📅 **工作日识别**：自动识别工作日、周末、法定节假日和调休补班日
- 🚀 **GitHub 集成**：自动获取当天创建的 Pull Request
- 🤖 **AI 生成**：使用 OpenAI API 自动生成格式化的站会报告
- 📱 **飞书通知**：通过自定义机器人 Webhook 发送到飞书群聊
- ⚡ **无服务器**：基于 Cloudflare Workers，无需管理服务器

## 部署步骤

### 1. 安装 Wrangler CLI

```bash
npm install -g wrangler
```

### 2. 登录 Cloudflare 账户

```bash
wrangler login
```

### 3. 克隆并准备项目

```bash
git clone <repository-url>
cd auto_daily_stand-up_meeting
```

### 4. 配置环境变量

在 Cloudflare Workers 仪表板中设置以下环境变量，或使用 wrangler 命令行：

#### 必需环境变量

```bash
# GitHub Token（必需）
wrangler secret put GITHUB_TOKEN
# 输入你的 GitHub Personal Access Token

# 飞书 Webhook URL（必需）
wrangler secret put FEISHU_WEBHOOK_URL
# 输入飞书自定义机器人的 Webhook URL
```

#### 可选环境变量（AI 功能）

```bash
# OpenAI API Key（可选，用于 AI 生成报告）
wrangler secret put OPENAI_API_KEY
# 输入你的 OpenAI API Key

# OpenAI Base URL（可选，默认为 OpenAI 官方）
wrangler secret put OPENAI_BASE_URL
# 例如：https://api.openai.com/v1

# OpenAI Model（可选，默认为 gpt-3.5-turbo）
wrangler secret put OPENAI_MODEL
# 例如：gpt-3.5-turbo 或 gpt-4
```

### 5. 部署到 Cloudflare Workers

```bash
wrangler deploy
```

## 环境变量详细说明

### GitHub Token 获取

1. 访问 [GitHub Settings > Personal access tokens](https://github.com/settings/tokens)
2. 点击 "Generate new token (classic)"
3. 选择适当的权限：
   选择 `repo`
4. 复制生成的 token

### 飞书自定义机器人 Webhook

1. 在飞书群聊中添加自定义机器人
2. 选择"自定义机器人"
3. 配置机器人名称和描述
4. 获取 Webhook URL（格式类似：`https://open.feishu.cn/open-apis/bot/v2/hook/xxxxxxxxxx`）

### OpenAI API Key（可选）

1. 访问 [OpenAI API Keys](https://platform.openai.com/api-keys)
2. 创建新的 API Key
3. 复制 API Key

## 定时任务设置

项目配置为每天北京时间下午 6 点检查工作日状态：

```toml
# wrangler.toml 中的配置
[triggers]
crons = ["0 10 * * *"]  # UTC 时间上午 10 点 = 北京时间下午 6 点
```

### 工作日判断逻辑

系统会自动调用中国节假日API来判断当前日期的工作状态：
- **0**: 普通工作日 → 执行站会报告
- **1**: 周末双休日 → 跳过执行  
- **2**: 需要补班的工作日 → 执行站会报告
- **3**: 法定节假日 → 跳过执行

这样确保了即使在调休期间（如国庆长假需要补班），系统也能正确判断是否应该执行站会报告。

如需修改执行时间，请编辑 `wrangler.toml` 文件中的 cron 表达式。

## 手动触发

部署后，您可以通过以下方式进行测试：

```bash
# 手动触发报告生成（不检查工作日状态）
curl https://your-worker-name.your-subdomain.workers.dev/manual-trigger

# 检查今天是否为工作日
curl https://your-worker-name.your-subdomain.workers.dev/check-working-day

# 健康检查
curl https://your-worker-name.your-subdomain.workers.dev/health
```

## 监控和日志

### 查看日志

```bash
wrangler tail
```

### 在线监控

1. 登录 [Cloudflare Dashboard](https://dash.cloudflare.com)
2. 进入 Workers & Pages 
3. 选择你的 Worker
4. 查看 Metrics 和 Logs

## 报告格式

生成的站会报告格式如下：

```
## 今日完成工作
[1]PROJECT_NAME#123-具体工作内容描述

## 下个工作日计划工作
- 继续优化某功能
- 完善某模块稳定性

## 遇到的障碍或需要帮助的事项
- 无特殊障碍
```

## 故障排除

### 常见问题

1. **GitHub API 限制**
   - 确保 GitHub Token 有效且权限足够
   - GitHub API 有速率限制，但个人使用通常不会触及

2. **AI API 调用失败**
   - 检查 OpenAI API Key 是否有效
   - 确认 API 余额是否充足
   - 检查网络连接是否正常

3. **飞书消息发送失败**
   - 确认 Webhook URL 格式正确
   - 检查机器人是否已被移除或禁用
   - 确认群聊中机器人权限正常

4. **定时任务不执行**
   - 确认 cron 表达式格式正确
   - 检查 Cloudflare Workers 计划是否激活
   - 查看 Workers 日志排查错误

### 调试技巧

1. **查看实时日志**：
   ```bash
   wrangler tail
   ```

2. **手动测试**：
   ```bash
   curl https://your-worker.workers.dev/manual-trigger
   ```

3. **检查环境变量**：
   ```bash
   wrangler secret list
   ```

## 成本估算

Cloudflare Workers 免费套餐包括：
- 每天 100,000 次请求
- 每次执行最多 10ms CPU 时间

对于每日一次的站会报告生成，完全在免费套餐范围内。

## 安全考虑

1. **敏感信息保护**：所有 API Key 和 Token 通过 Cloudflare Workers 的 Secret 功能加密存储
2. **网络安全**：Cloudflare Workers 运行在安全的沙箱环境中
3. **权限最小化**：GitHub Token 只需要必要的仓库访问权限

## 升级和维护

### 更新代码

```bash
# 拉取最新代码
git pull origin main

# 重新部署
wrangler deploy
```

### 更新环境变量

```bash
# 更新某个环境变量
wrangler secret put VARIABLE_NAME
```

### 监控运行状态

建议定期检查：
- Workers 执行日志
- GitHub API 配额使用情况
- OpenAI API 余额
- 飞书机器人状态

## 联系和支持

如果遇到问题或需要技术支持，请：
1. 查看 Workers 执行日志
2. 检查环境变量配置
3. 确认第三方服务（GitHub、OpenAI、飞书）状态正常 