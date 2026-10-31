"use client";

import { useCallback, useEffect, useState } from "react";
import { Bot, Coins, ListChecks, ShieldAlert, UserRoundPlus, Wrench } from "lucide-react";
import { get } from "@/lib/api";
import { Card, CardHeader, ErrorText, MetricCard, Spinner, StatusBadge } from "@/components/ui";

export default function DashboardPage() {
  const [summary, setSummary] = useState<any>(null);
  const [report, setReport] = useState<string>("");
  const [approvals, setApprovals] = useState<any[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [s, r, a] = await Promise.all([
        get<any>("/api/v1/analytics/summary"),
        get<any>("/api/v1/analytics/daily-report"),
        get<any>("/api/v1/approvals?status=PENDING"),
      ]);
      setSummary(s);
      setReport(r.markdown || "");
      setApprovals(a.items || []);
      setError(null);
    } catch (e: any) {
      setError(e.message);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  if (error) return <div className="p-6"><ErrorText message={error} /></div>;
  if (!summary) return <Spinner />;

  return (
    <div className="p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">经营总览</h1>
          <p className="text-xs text-muted-foreground">虚拟员工团队今日工作情况</p>
        </div>
        <button onClick={load} className="text-xs text-muted-foreground hover:text-foreground">刷新</button>
      </div>

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <MetricCard icon={<Bot size={16} />} value={summary.agents} label="在职 Agent" description={`工具 ${summary.tools} 个已注册`} />
        <MetricCard icon={<ListChecks size={16} />} value={summary.tasks.completed} label="完成任务" description={`失败 ${summary.tasks.failed} · 进行中 ${summary.tasks.active}`} />
        <MetricCard icon={<UserRoundPlus size={16} />} value={summary.leads.total} label="客户线索" description={`新线索 ${summary.leads.new} · 成交 ${summary.leads.won}`} />
        <MetricCard icon={<Coins size={16} />} value={summary.tokens_used} label="Token 消耗" description="全部任务累计" />
      </div>

      <div className="mt-4 grid gap-4 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader title="今日经营日报" desc="Data Analyst Agent 自动生成" />
          <pre className="whitespace-pre-wrap px-4 py-3 font-mono text-xs leading-relaxed text-muted-foreground">{report || "暂无"}</pre>
        </Card>
        <Card>
          <CardHeader title={<span className="flex items-center gap-1.5"><ShieldAlert size={14} /> 待审批</span>} desc={`${summary.approvals_pending} 项等待你的决定`} />
          <div className="divide-y">
            {approvals.length === 0 ? <div className="px-4 py-6 text-center text-xs text-muted-foreground">没有待办 ✅</div> : null}
            {approvals.slice(0, 6).map((a) => (
              <div key={a.id} className="flex items-center justify-between gap-2 px-4 py-2.5">
                <div className="min-w-0">
                  <div className="truncate text-xs">{a.action}</div>
                  <div className="text-[10px] text-muted-foreground">by {a.requested_by}</div>
                </div>
                <StatusBadge status="PENDING" />
              </div>
            ))}
          </div>
        </Card>
      </div>

      <div className="mt-4 grid gap-4 md:grid-cols-3">
        <MetricCard icon={<ListChecks size={16} />} value={summary.content.topics} label="内容选题" description={`内容资产 ${summary.content.assets} · 已批 ${summary.content.approved}`} />
        <MetricCard icon={<Wrench size={16} />} value={summary.publishing.human_queue} label="人工发布队列" description={`已发布 ${summary.publishing.published}`} />
        <MetricCard icon={<ShieldAlert size={16} />} value={summary.memories.pending_review} label="记忆待确认" description={`生效记忆 ${summary.memories.total}`} />
      </div>
    </div>
  );
}
