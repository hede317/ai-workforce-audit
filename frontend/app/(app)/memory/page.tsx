"use client";

import { useCallback, useEffect, useState } from "react";
import { BrainCircuit, Plus } from "lucide-react";
import { get, post, put } from "@/lib/api";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner } from "@/components/ui";

const KINDS = ["ENTERPRISE", "USER", "TASK", "KNOWLEDGE"] as const;

export default function MemoryPage() {
  const [items, setItems] = useState<any[]>([]);
  const [kind, setKind] = useState("");
  const [q, setQ] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [form, setForm] = useState({ kind: "ENTERPRISE", title: "", content: "" });

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const params = new URLSearchParams();
      if (kind) params.set("kind", kind);
      if (q) params.set("q", q);
      const d = await get<any>(`/api/v1/memories?${params.toString()}`);
      setItems(d.items || []);
      setError(null);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [kind, q]);

  useEffect(() => {
    load();
  }, [load]);

  async function confirmPending(id: string) {
    await put(`/api/v1/memories/${id}`, { status: "ACTIVE" });
    load();
  }
  async function archive(id: string) {
    await put(`/api/v1/memories/${id}`, { status: "ARCHIVED" });
    load();
  }
  async function add() {
    if (!form.title || !form.content) return;
    await post("/api/v1/memories", form);
    setForm({ kind: form.kind, title: "", content: "" });
    setAdding(false);
    load();
  }

  return (
    <div className="p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h1 className="flex items-center gap-2 text-xl font-bold">
            <BrainCircuit size={20} /> 记忆中心
          </h1>
          <p className="text-xs text-muted-foreground">企业记忆 / 用户偏好 / 任务记忆 / 专业知识库——Agent 不会每次重新认识你</p>
        </div>
        <Button onClick={() => setAdding(!adding)} variant={adding ? "secondary" : "default"}>
          <Plus size={14} /> 新增记忆
        </Button>
      </div>

      <div className="mb-3 flex gap-2">
        <select className="h-8 rounded-md border bg-background px-2 text-xs" value={kind} onChange={(e) => setKind(e.target.value)}>
          <option value="">全部类型</option>
          {KINDS.map((k) => (
            <option key={k} value={k}>
              {k}
            </option>
          ))}
        </select>
        <input
          className="h-8 flex-1 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
          placeholder="检索记忆内容…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
      </div>

      {adding ? (
        <Card className="mb-3 p-4">
          <div className="flex gap-2">
            <select className="h-8 rounded-md border bg-background px-2 text-xs" value={form.kind} onChange={(e) => setForm({ ...form, kind: e.target.value })}>
              {KINDS.map((k) => (
                <option key={k} value={k}>
                  {k}
                </option>
              ))}
            </select>
            <input className="h-8 flex-1 rounded-md border bg-background px-3 text-xs" placeholder="标题" value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} />
          </div>
          <textarea
            className="mt-2 min-h-[64px] w-full rounded-md border bg-background p-2 text-xs outline-none focus:ring-[3px] focus:ring-ring"
            placeholder="内容（企业记忆由你写入即生效；Agent 写入的需要你确认）"
            value={form.content}
            onChange={(e) => setForm({ ...form, content: e.target.value })}
          />
          <div className="mt-2 flex gap-2">
            <Button size="sm" onClick={add}>
              保存
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setAdding(false)}>
              取消
            </Button>
          </div>
        </Card>
      ) : null}

      <ErrorText message={error} />
      {loading ? (
        <Spinner />
      ) : items.length === 0 ? (
        <Card>
          <Empty text="暂无记忆" />
        </Card>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          {items.map((m) => (
            <Card key={m.id}>
              <CardHeader
                title={
                  <span className="flex items-center gap-2">
                    <Badge tone={m.kind === "ENTERPRISE" ? "indigo" : m.kind === "TASK" ? "neutral" : "blue"}>{m.kind}</Badge>
                    <span className="truncate">{m.title}</span>
                  </span>
                }
                action={<StatusActions m={m} onConfirm={confirmPending} onArchive={archive} />}
              />
              <p className="px-4 py-3 text-xs leading-relaxed text-muted-foreground">{m.content}</p>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

function StatusActions({ m, onConfirm, onArchive }: { m: any; onConfirm: (id: string) => void; onArchive: (id: string) => void }) {
  if (m.status === "PENDING") {
    return (
      <span className="flex shrink-0 items-center gap-1">
        <Badge tone="yellow">待确认</Badge>
        <Button size="sm" variant="success" onClick={() => onConfirm(m.id)}>
          采纳
        </Button>
      </span>
    );
  }
  if (m.status === "ACTIVE") {
    return (
      <span className="shrink-0">
        <Badge tone="green">生效</Badge>{" "}
        <Button size="sm" variant="ghost" onClick={() => onArchive(m.id)} title="归档">
          归档
        </Button>
      </span>
    );
  }
  return <Badge tone="neutral">{m.status}</Badge>;
}
