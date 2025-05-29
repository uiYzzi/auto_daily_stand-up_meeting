# 自动化每日站会报告生成器

这个 Rust 项目可以自动获取您在 GitHub 上今天创建的 Pull Request，并生成符合公司规定格式的每日站会报告内容。

## 功能特性

- 🚀 自动获取当天创建的 PR
- 📅 支持查询指定日期的 PR
- 🔐 GitHub Token 认证（确保安全性和无频率限制）
- 🤖 **AI 自动生成站会报告**（支持 OpenAI 及兼容 API）
- 📊 智能提取项目代号和 Taiga issue 信息

## 安装和使用

### 1. 编译项目

```bash
git clone <repository-url>
cd auto_daily_stand-up_meeting
cargo build --release
```

### 2. 配置

在项目根目录创建 `.env` 文件：

```bash
# GitHub 配置（必需）
GITHUB_TOKEN=your_github_token_here

# AI 配置（可选，支持自动生成站会报告）
OPENAI_API_KEY=your_openai_api_key
OPENAI_BASE_URL=https://api.openai.com/v1
OPENAI_MODEL=gpt-3.5-turbo
```

#### GitHub Token 获取：
1. 访问 [GitHub Settings > Personal access tokens](https://github.com/settings/tokens)
2. 创建新 token，选择适当权限（公开仓库选 `public_repo`，私有仓库选 `repo`）
3. 复制 token 到 `.env` 文件

#### OpenAI API Key 获取（可选）：
访问 [OpenAI API Keys](https://platform.openai.com/api-keys) 创建新的 API Key

### 3. 运行

```bash
cargo run
```

## 输出示例

### 配置了 AI 的输出

```
🤖 AI 生成的每日站会报告：
======================================
## 今日完成工作
[1]XXXX#22-XXXXXXXXXX
[1]XXXX#17-XXXXXXXXXXXXX

## 下个工作日计划工作
- 继续优化 XXXXXX 功能
- 完善XXXXX稳定性

## 遇到的障碍或需要帮助的事项
- 无特殊障碍
======================================
```

### 未配置 AI 的输出

如果没有配置 AI，程序会输出原始数据供您复制给其他 AI 助手处理。

## 许可证

MIT License 