"use client";

import { useEffect, useState } from "react";
import { Play, Send, Square } from "lucide-react";
import { get, post } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner, StatusBadge } from "@/components/ui";

interface ChildTask {
  id: string;
  title: string;
  agent_code: string;
  status: string;
  tool_hint?: string;
  error?: string;
  result?: any;
}

export default function CommanderPage() {
  const t = useT();
  const [goal, setGoal] = useState("");
  const [bu, setBu] = useState("");
  const [bus, setBus] = useState<any[]>([]);
  const [plan, setPlan] = useState<{ root_id: string; mode: string; children: any[] } | null>(null);
  const [tree, setTree] = useState<{ root: any; children: ChildTask[] } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 审计#5(P0)：渲染期发起请求=setState死循环打爆后端，必须挂载时仅一次
  useEffect(() => {
    let active = true;
    get<any>("/api/v1/business-units")
      .then((d) => {
        if (!active) return;
        setBus(d.items || []);
        if (d.items?.length) setBu((prev) => prev || d.items[0].code);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, []);

  async function submitGoal() {
    if (!goal.trim()) return;
    setBusy(true);
    setError(null);
    setPlan(null);
    setTree(null);
    try {
      const data = await post<any>("/api/v1/commander/goals", { goal, business_unit_code: bu || undefined });
      setPlan(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }

  async function execute() {
    if (!plan) return;
    setBusy(true);
    setError(null);
    try {
      await post(`/api/v1/commander/goals/${plan.root_id}/execute`, { sync: true });
      const t = await get<any>(`/api/v1/commander/goals/${plan.root_id}`);
      setTree(t);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }

  async function refresh() {
    if (!plan) return;
    const t = await get<any>(`/api/v1/commander/goals/${plan.root_id}`);
    setTree(t);
  }

  return (
    <div className="p-6">
      <h1 className="text-xl font-bold">AI Commander</h1>
      <p className="mb-4 text-xs text-muted-foreground">{t("cmd.sub")}</p>

      <Card className="p-4">
        <textarea
          className="min-h-[72px] w-full resize-y rounded-md border bg-background p-3 text-sm outline-none focus:ring-[3px] focus:ring-ring"
          placeholder={t("cmd.ph")}
          value={goal}
          onChange={(e) => setGoal(e.target.value)}
        />
        <div className="mt-3 flex items-center gap-2">
          <select
            className="h-9 rounded-md border bg-background px-2 text-sm outline-none"
            value={bu}
            onChange={(e) => setBu(e.target.value)}
          >
            {bus.map((b) => (
              <option key={b.code} value={b.code}>
                {b.code} · {b.name}
              </option>
            ))}
          </select>
          <Button onClick={submitGoal} disabled={busy || !goal.trim()}>
            <Send size={14} /> {t("cmd.send")}
          </Button>
          {plan ? (
            <Button variant="secondary" onClick={execute} disabled={busy}>
              <Play size={14} /> {t("cmd.exec")}
            </Button>
          ) : null}
          {tree ? (
            <Button variant="ghost" onClick={refresh} disabled={busy}>
              {t("cmd.refreshStatus")}
            </Button>
          ) : null}
        </div>
        <ErrorText message={error} />
      </Card>

      {busy ? <Spinner text={t("cmd.working")} /> : null}

      {plan ? (
        <Card className="mt-4">
          <CardHeader
            title={t("cmd.plan")}
            desc={
              <span className="flex items-center gap-2">
                <Badge tone={plan.mode === "ai" ? "indigo" : "neutral"}>{plan.mode === "ai" ? t("cmd.modeAi") : t("cmd.modeOffline")}</Badge>
                root: <span className="font-mono text-[10px]">{plan.root_id.slice(0, 12)}</span>
              </span>
            }
          />
          <div className="divide-y">
            {plan.children.map((c: any) => (
              <div key={c.id} className="flex items-center justify-between gap-3 px-4 py-2.5">
                <div className="min-w-0">
                  <div className="truncate text-sm">{c.title}</div>
                  <div className="text-xs text-muted-foreground">
                    {c.agent_code}
                    {c.tool_hint ? ` · ${t("cmd.tool")} ${c.tool_hint}` : ` · ${t("cmd.aiFree")}`}
                  </div>
                </div>
                {c.requires_approval ? <Badge tone="yellow">{t("cmd.needApproval")}</Badge> : <Badge tone="neutral">{t("cmd.auto")}</Badge>}
              </div>
            ))}
          </div>
        </Card>
      ) : null}

      {tree ? (
        <Card className="mt-4">
          <CardHeader title={t("cmd.result")} desc={<StatusBadge status={tree.root.status} />} />
          {tree.children.length === 0 ? <Empty /> : null}
          <div className="divide-y">
            {tree.children.map((c) => (
              <div key={c.id} className="px-4 py-3">
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="truncate text-sm">{c.title}</div>
                    <div className="text-xs text-muted-foreground">{c.agent_code}</div>
                  </div>
                  <StatusBadge status={c.status} />
                </div>
                {c.error ? <div className="mt-1.5 rounded-md bg-destructive/10 px-2 py-1 font-mono text-[10px] text-[oklch(0.75_0.15_25)]">{c.error}</div> : null}
                {c.result?.output ? (
                  <details className="mt-1.5">
                    <summary className="cursor-pointer text-xs text-muted-foreground hover:text-foreground">{t("cmd.viewOutput")}</summary>
                    <pre className="mt-1 max-h-48 overflow-auto whitespace-pre-wrap rounded-md bg-[oklch(0.955_0.02_205)] p-3 font-mono text-[11px] leading-relaxed">{String(c.result.output)}</pre>
                  </details>
                ) : null}
              </div>
            ))}
          </div>
        </Card>
      ) : null}
    </div>
  );
}
