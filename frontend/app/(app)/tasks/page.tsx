"use client";

import { useCallback, useEffect, useState } from "react";
import { RotateCcw, Square } from "lucide-react";
import { get, post } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Button, Card, Empty, ErrorText, Spinner, StatusBadge } from "@/components/ui";

export default function TasksPage() {
  const tr = useT();
  const [tasks, setTasks] = useState<any[]>([]);
  const [status, setStatus] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [open, setOpen] = useState<string | null>(null);
  const [detail, setDetail] = useState<any>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const d = await get<any>(`/api/v1/tasks${status ? `?status=${encodeURIComponent(status)}` : ""}`);
      setTasks(d.items || []);
      setError(null);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [status]);

  useEffect(() => {
    load();
  }, [load]);

  async function showDetail(id: string) {
    if (open === id) {
      setOpen(null);
      return;
    }
    const d = await get<any>(`/api/v1/tasks/${id}`);
    setDetail(d);
    setOpen(id);
  }

  async function retry(id: string) {
    await post(`/api/v1/tasks/${id}/retry`);
    load();
  }
  async function cancel(id: string) {
    await post(`/api/v1/tasks/${id}/cancel`);
    load();
  }

  return (
    <div className="p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">{tr("tasks.title")}</h1>
          <p className="text-xs text-muted-foreground">{tr("tasks.sub")}</p>
        </div>
        <select className="h-8 rounded-md border bg-background px-2 text-xs" value={status} onChange={(e) => setStatus(e.target.value)}>
          <option value="">{tr("tasks.allStatus")}</option>
          {["Created", "Running", "Waiting Approval", "Completed", "Failed", "Cancelled", "Rejected"].map((s) => (
            <option key={s} value={s}>
              {tr(s)}
            </option>
          ))}
        </select>
      </div>

      <ErrorText message={error} />
      <Card>
        {loading ? (
          <Spinner />
        ) : tasks.length === 0 ? (
          <Empty text={tr("tasks.empty")} />
        ) : (
          <div className="divide-y">
            {tasks.map((t) => (
              <div key={t.id}>
                <div className="flex items-center justify-between gap-3 px-4 py-2.5 hover:bg-accent/40">
                  <button className="min-w-0 flex-1 text-left" onClick={() => showDetail(t.id)}>
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm">{t.title}</span>
                      {t.depth === 0 ? <span className="rounded bg-muted px-1 py-0.5 text-[10px] text-muted-foreground">{tr("tasks.goalTag")}</span> : null}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {t.agent_code || "-"}
                      {t.tool_hint ? ` · ${t.tool_hint}` : ""}
                      {` · ${tr("tasks.tokens", { n: t.tokens_used })}`}
                    </div>
                  </button>
                  <div className="flex shrink-0 items-center gap-1.5">
                    {t.status === "Failed" ? (
                      <Button size="sm" variant="secondary" onClick={() => retry(t.id)}>
                        <RotateCcw size={12} /> {tr("c.retry")}
                      </Button>
                    ) : null}
                    {["Created", "Waiting Approval", "Assigned", "Planning"].includes(t.status) ? (
                      <Button size="sm" variant="ghost" onClick={() => cancel(t.id)}>
                        <Square size={12} /> {tr("c.cancel")}
                      </Button>
                    ) : null}
                    <StatusBadge status={t.status} />
                  </div>
                </div>
                {open === t.id && detail ? (
                  <div className="bg-[oklch(0.955_0.02_205)] px-4 py-3">
                    <pre className="max-h-64 overflow-auto whitespace-pre-wrap font-mono text-[11px] leading-relaxed text-muted-foreground">
                      {JSON.stringify(detail, null, 2)}
                    </pre>
                  </div>
                ) : null}
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
