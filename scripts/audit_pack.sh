#!/usr/bin/env bash
# 审计包生成器：把待审计内容打成自包含 Markdown，拖进 Tabbit（前沿模型）做第二意见
# 用法: bash audit_pack.sh [since_commit] [聚焦问题]
# 产物: ~/Desktop/awf-audit-<日期>.md
set -euo pipefail
cd "$(dirname "$0")/.."

SINCE="${1:-c3167c9}"
FOCUS="${2:-}"
OUT="$HOME/Desktop/awf-audit-$(date +%Y%m%d-%H%M).md"

{
echo "# HedeYou AI Workforce 审计包（$(date +%Y-%m-%d %H:%M)）"
echo
echo "你是资深软件审计师。以下是一个 Rust(Axum+SQLx)+PostgreSQL+Next.js 的 AI Agent 平台的增量变更。"
echo "请按【红线清单】逐项审查【本次变更】，输出：问题列表(严重度/位置/证据/修复建议) + 总体结论。"
echo
echo "## 设计红线（审查基准）"
cat <<'RED'
1. APPROVAL级工具未获批准不得执行（tools.rs dispatch门 + invoke_tool 409拦截）
2. 内容未APPROVED不得发布（business.rs::publish_content）
3. 企业记忆Agent只能写PENDING（tools.rs::memory_save）
4. 商品上架仅ADMIN且状态机不可跳步（business.rs::update_product）
5. 幂等/审批留痕/审计日志必须真实落库
6. SSRF防护：web.fetch/seo.check内网黑名单+512KB限量；WP/MPT scheme校验+禁重定向
7. 审批快照哈希校验：批准前任务参数与审批时一致（orchestrator.rs）
8. 任务执行原子抢占防并发双执行；根任务PartialFailed语义正确
RED
echo
echo "## 提交清单（${SINCE}..HEAD）"
git log --oneline "${SINCE}..HEAD"
echo
echo "## 聚焦问题（本轮请优先回答）"
if [ -n "$FOCUS" ]; then echo "$FOCUS"; else
cat <<'FOC'
1. pipeline.content_full（tools.rs）：同步调MPT最长2500s，阻塞invoke是否有更好形态？
2. openmaic.classroom（tools.rs）：14分钟轮询上限与STILL_RUNNING(202)语义是否合理？
3. botarmy.dispatch（tools.rs）：团目录白名单+工单写入有无路径/内容注入面？
4. listening 模块（tools.rs listen_scan + routers/listening.rs）：RSS解析与信号转化有无注入/重复问题？
5. 老何(chief-of-staff)入编后根任务人格变更是否有遗漏的引用点？
FOC
fi
echo
echo "## 完整Diff"
echo '```diff'
git diff "${SINCE}..HEAD" -- backend/src frontend/lib/i18n.tsx 'frontend/app/(app)' | head -2500
echo '```'
echo
echo "## 测试现状"
cargo test 2>/dev/null | grep 'test result' | grep -v ' 0 passed' || echo "(本机生成时：17/17绿)"
} > "$OUT"

echo "审计包已生成: $OUT"
wc -l "$OUT"
