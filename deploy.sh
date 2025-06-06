#!/bin/bash

# Cloudflare Workers 自动部署脚本
# 用于自动化每日站会报告生成器的部署

set -e

echo "=========================================="
echo "🚀 自动化每日站会报告生成器 - Cloudflare Workers 部署"
echo "=========================================="

# 检查是否安装了 wrangler
if ! command -v wrangler &> /dev/null; then
    echo "❌ Wrangler CLI 未安装"
    echo "📥 正在安装 Wrangler CLI..."
    npm install -g wrangler
    echo "✅ Wrangler CLI 安装完成"
fi

# 检查是否已登录
echo "🔐 检查 Cloudflare 登录状态..."
if ! wrangler whoami &> /dev/null; then
    echo "📋 请先登录 Cloudflare 账户"
    wrangler login
fi

echo "✅ Cloudflare 账户已登录"

# 部署到 Cloudflare Workers
echo "🚀 正在部署到 Cloudflare Workers..."
wrangler deploy

echo ""
echo "=========================================="
echo "✅ 部署完成！"
echo "=========================================="
echo ""
echo "📋 下一步需要配置环境变量："
echo ""
echo "🔑 必需的环境变量："
echo "   wrangler secret put GITHUB_TOKEN"
echo "   wrangler secret put FEISHU_WEBHOOK_URL"
echo ""
echo "🤖 可选的 AI 环境变量："
echo "   wrangler secret put OPENAI_API_KEY"
echo "   wrangler secret put OPENAI_BASE_URL"
echo "   wrangler secret put OPENAI_MODEL"
echo ""
echo "📖 详细配置指南请查看：CLOUDFLARE_DEPLOYMENT.md"
echo ""
echo "📊 查看日志："
echo "   wrangler tail"
echo ""
echo "⏰ 定时任务：每天北京时间下午 7 点自动执行"
echo "==========================================" 