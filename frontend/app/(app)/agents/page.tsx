"use client";

import { useCallback, useEffect, useState } from "react";
import { Check, Pencil, X } from "lucide-react";
import { get, put } from "@/lib/api";
import { Badge, Button, Card, CardHeader, ErrorText, Spinner, StatusDot } from "@/components/ui";

export default function AgentsPage() {
  const [agents, setAgents] = useState<any[]>([]);
  const [tools, setTools] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    try {
      const [a, t] = await Promise.all([get<any>("/api/v1/agents"), get<any>("/api/v1/tools")]);
      setAgents(a.items || []);
      setTools(t.items || []);
      setError(null);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  function startEdit(a: any) {
    setEditing(a.id);
    setPrompt(a.system_prompt || "");
  }

  async function save(a: any) {
    setSaving(true);
    try {
      await put(`/api/v1/agents/${a.id}`, { system_prompt: prompt });
      setEditing(null);
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  if (loading) return <Spinner />;

  return (
    <div className="p-6">
      <h1 className="text-xl font-bold">Agent 团队</h1>
      <p className="mb-4 text-xs text-muted-foreground">1 名总经理 + 11 名专业虚拟员工，每个 Agent 有独立职责、系统提示词、工具权限与执行预算</p>
      <ErrorText message={error} />

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        {agents.map((a) => (
          <Card key={a.id}>
            <CardHeader
              title={
                <span className="flex items-center gap-2">
                  <StatusDot status={a.status} /> {a.name}
                  <span className="font-mono text-[10px] text-muted-foreground">{a.code}</span>
                </span>
              }
              desc={a.role}
              action={
                editing === a.id ? (
                  <span className="flex gap-1">
                    <Button size="sm" variant="success" onClick={() => save(a)} disabled={saving}>
                      <Check size={12} />
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => setEditing(null)}>
                      <X size={12} />
                    </Button>
                  </span>
                ) : (
                  <Button size="sm" variant="ghost" onClick={() => startEdit(a)} title="编辑系统提示词">
                    <Pencil size={12} />
                  </Button>
                )
              }
            />
            <div className="px-4 py-3">
              <p className="text-xs leading-relaxed text-muted-foreground">{a.responsibilities}</p>
              <div className="mt-2 flex flex-wrap gap-1">
                {(a.tools || []).map((t: string) => (
                  <Badge key={t} tone="blue">
                    {t}
                  </Badge>
                ))}
              </div>
              <div className="mt-2 text-[10px] text-muted-foreground">预算：{a.max_tool_calls} 次工具 · {a.max_tokens} tokens · {a.max_seconds}s</div>
              {editing === a.id ? (
                <textarea
                  className="mt-2 min-h-[96px] w-full rounded-md border bg-background p-2 font-mono text-[11px] outline-none focus:ring-[3px] focus:ring-ring"
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                />
              ) : null}
            </div>
          </Card>
        ))}
      </div>

      <h2 className="mb-2 mt-8 text-lg font-semibold">工具注册表</h2>
      <Card>
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b bg-accent/20 text-left text-muted-foreground">
              <th className="px-4 py-2 font-medium">工具</th>
              <th className="px-4 py-2 font-medium">分类</th>
              <th className="px-4 py-2 font-medium">说明</th>
              <th className="px-4 py-2 font-medium">权限</th>
            </tr>
          </thead>
          <tbody>
            {tools.map((t) => (
              <tr key={t.id} className="border-b last:border-0 hover:bg-accent/30">
                <td className="px-4 py-2 font-mono">{t.code}</td>
                <td className="px-4 py-2">{t.category}</td>
                <td className="px-4 py-2 text-muted-foreground">{t.description}</td>
                <td className="px-4 py-2">
                  <Badge tone={t.permission_level === "APPROVAL" ? "yellow" : "green"}>{t.permission_level === "APPROVAL" ? "需审批" : "自动"}</Badge>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>
    </div>
  );
}
