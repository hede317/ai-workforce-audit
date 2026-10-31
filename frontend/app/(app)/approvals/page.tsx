"use client";

import { useCallback, useEffect, useState } from "react";
import { Check, ShieldCheck, X } from "lucide-react";
import { get, post } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Badge, Button, Card, Empty, ErrorText, Spinner, StatusBadge } from "@/components/ui";

export default function ApprovalsPage() {
  const t = useT();
  const [items, setItems] = useState<any[]>([]);
  const [status, setStatus] = useState("PENDING");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const d = await get<any>(`/api/v1/approvals?status=${status}`);
      setItems(d.items || []);
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

  async function decide(id: string, decision: "APPROVED" | "REJECTED") {
    try {
      await post(`/api/v1/approvals/${id}/decide`, { decision, note: decision === "APPROVED" ? "已批准" : "已拒绝" });
      load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h1 className="flex items-center gap-2 text-xl font-bold">
            <ShieldCheck size={20} /> {t("ap.title")}
          </h1>
          <p className="text-xs text-muted-foreground">{t("ap.sub")}</p>
        </div>
        <select className="h-8 rounded-md border bg-background px-2 text-xs" value={status} onChange={(e) => setStatus(e.target.value)}>
          {["PENDING", "APPROVED", "REJECTED"].map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </div>

      <ErrorText message={error} />
      <Card>
        {loading ? (
          <Spinner />
        ) : items.length === 0 ? (
          <Empty text={status === "PENDING" ? t("ap.allClear") : t("ap.empty")} />
        ) : (
          <div className="divide-y">
            {items.map((a) => (
              <div key={a.id} className="flex items-center justify-between gap-3 px-4 py-3 hover:bg-accent/40">
                <div className="min-w-0">
                  <div className="truncate text-sm">{a.action}</div>
                  <div className="text-xs text-muted-foreground">
                    {t("ap.task")} <span className="font-mono">{a.task_id.slice(0, 12)}</span> · {t("ap.by")} {a.requested_by || "-"}
                    {a.decided_by ? ` · ${t("ap.decidedBy")} ${a.decided_by}` : ""}
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-1.5">
                  <Badge tone={a.status === "PENDING" ? "yellow" : a.status === "APPROVED" ? "green" : "red"}>{a.status}</Badge>
                  {a.status === "PENDING" ? (
                    <>
                      <Button size="sm" variant="success" onClick={() => decide(a.id, "APPROVED")}>
                        <Check size={12} /> {t("c.approve")}
                      </Button>
                      <Button size="sm" variant="destructive" onClick={() => decide(a.id, "REJECTED")}>
                        <X size={12} /> {t("c.reject")}
                      </Button>
                    </>
                  ) : (
                    <StatusBadge status={a.status === "APPROVED" ? "APPROVED" : "Rejected"} />
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
