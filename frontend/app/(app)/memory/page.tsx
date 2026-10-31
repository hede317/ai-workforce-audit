"use client";

import { useCallback, useEffect, useState } from "react";
import { BrainCircuit, Plus } from "lucide-react";
import { get, post, put } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner } from "@/components/ui";

const KINDS = ["ENTERPRISE", "USER", "TASK", "KNOWLEDGE"] as const;

export default function MemoryPage() {
  const t = useT();
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
    try {
      await put(`/api/v1/memories/${id}`, { status: "ACTIVE" });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }
  async function archive(id: string) {
    try {
      await put(`/api/v1/memories/${id}`, { status: "ARCHIVED" });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }
  async function add() {
    if (!form.title || !form.content) return;
    try {
      await post("/api/v1/memories", form);
      setForm({ kind: form.kind, title: "", content: "" });
      setAdding(false);
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h1 className="flex items-center gap-2 text-xl font-bold">
            <BrainCircuit size={20} /> {t("mem.title")}
          </h1>
          <p className="text-xs text-muted-foreground">{t("mem.sub")}</p>
        </div>
        <Button onClick={() => setAdding(!adding)} variant={adding ? "secondary" : "default"}>
          <Plus size={14} /> {t("mem.add")}
        </Button>
      </div>

      <div className="mb-3 flex gap-2">
        <select className="h-8 rounded-md border bg-background px-2 text-xs" value={kind} onChange={(e) => setKind(e.target.value)}>
          <option value="">{t("mem.allTypes")}</option>
          {KINDS.map((k) => (
            <option key={k} value={k}>
              {k}
            </option>
          ))}
        </select>
        <input
          className="h-8 flex-1 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
          placeholder={t("mem.searchPh")}
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
            <input className="h-8 flex-1 rounded-md border bg-background px-3 text-xs" placeholder={t("mem.phTitle")} value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} />
          </div>
          <textarea
            className="mt-2 min-h-[64px] w-full rounded-md border bg-background p-2 text-xs outline-none focus:ring-[3px] focus:ring-ring"
            placeholder={t("mem.phContent")}
            value={form.content}
            onChange={(e) => setForm({ ...form, content: e.target.value })}
          />
          <div className="mt-2 flex gap-2">
            <Button size="sm" onClick={add}>
              {t("c.save")}
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setAdding(false)}>
              {t("c.cancel")}
            </Button>
          </div>
        </Card>
      ) : null}

      <ErrorText message={error} />
      {loading ? (
        <Spinner />
      ) : items.length === 0 ? (
        <Card>
          <Empty text={t("mem.empty")} />
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
  const t = useT();
  if (m.status === "PENDING") {
    return (
      <span className="flex shrink-0 items-center gap-1">
        <Badge tone="yellow">{t("PENDING")}</Badge>
        <Button size="sm" variant="success" onClick={() => onConfirm(m.id)}>
          {t("mem.confirm")}
        </Button>
      </span>
    );
  }
  if (m.status === "ACTIVE") {
    return (
      <span className="shrink-0">
        <Badge tone="green">{t("ACTIVE")}</Badge>{" "}
        <Button size="sm" variant="ghost" onClick={() => onArchive(m.id)} title={t("mem.archive")}>
          {t("mem.archive")}
        </Button>
      </span>
    );
  }
  return <Badge tone="neutral">{m.status}</Badge>;
}
