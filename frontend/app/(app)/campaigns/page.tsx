"use client";

import { useCallback, useEffect, useState } from "react";
import { Megaphone, Play } from "lucide-react";
import { get, post } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner } from "@/components/ui";

export default function CampaignsPage() {
  const [camps, setCamps] = useState<any[]>([]);
  const [execs, setExecs] = useState<any[]>([]);
  const [wfCode, setWfCode] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const t = useT();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [c, w] = await Promise.all([get<any>("/api/v1/campaigns"), get<any>("/api/v1/workflows/executions")]);
      setCamps(c.items || []);
      setExecs(w.items || []);
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

  async function run() {
    if (!wfCode.trim()) return;
    try {
      await post("/api/v1/workflows/trigger", { workflow_code: wfCode.trim(), payload: {} });
      setWfCode("");
      load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="p-6">
      <h1 className="flex items-center gap-2 text-xl font-bold">
        <Megaphone size={20} /> {t("mkt.title")}
      </h1>
      <p className="mb-4 text-xs text-muted-foreground">{t("mkt.sub")}</p>
      <ErrorText message={error} />

      <Card className="mb-4">
        <CardHeader title={t("mkt.camp")} />
        {camps.length === 0 ? (
          <Empty text={t("mkt.empty")} />
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="border-b bg-accent/20 text-left text-muted-foreground">
                <th className="px-4 py-2 font-medium">{t("crm.phName")}</th>
                <th className="px-4 py-2 font-medium">{t("mkt.chan")}</th>
                <th className="px-4 py-2 font-medium">{t("mkt.budget")}</th>
                <th className="px-4 py-2 font-medium">{t("mkt.spend")}</th>
                <th className="px-4 py-2 font-medium">{t("crm.status")}</th>
              </tr>
            </thead>
            <tbody>
              {camps.map((c) => (
                <tr key={c.id} className="border-b last:border-0 hover:bg-accent/30">
                  <td className="px-4 py-2">{c.name}</td>
                  <td className="px-4 py-2">{c.channel || "-"}</td>
                  <td className="px-4 py-2 font-mono">¥{c.budget}</td>
                  <td className="px-4 py-2 font-mono">¥{c.spend}</td>
                  <td className="px-4 py-2">
                    <Badge tone={c.status === "ACTIVE" ? "green" : "neutral"}>{c.status}</Badge>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>

      <Card>
        <CardHeader
          title={t("mkt.wf")}
          desc={t("mkt.wfDesc")}
          action={
            <div className="flex gap-1.5">
              <input className="h-7 w-44 rounded-md border bg-background px-2 text-xs outline-none" placeholder={t("mkt.wfCode")} value={wfCode} onChange={(e) => setWfCode(e.target.value)} />
              <Button size="sm" variant="secondary" onClick={run}>
                <Play size={12} /> {t("mkt.run")}
              </Button>
            </div>
          }
        />
        {execs.length === 0 ? (
          <Empty text={t("mkt.empty")} />
        ) : (
          <div className="divide-y">
            {execs.map((w) => (
              <div key={w.id} className="flex items-center justify-between gap-3 px-4 py-2.5">
                <div className="min-w-0">
                  <div className="truncate text-sm font-mono">{w.workflow_code}</div>
                  <div className="text-xs text-muted-foreground">{w.id.slice(0, 12)}</div>
                </div>
                <div className="flex items-center gap-1.5">
                  {w.error ? <span className="text-xs text-[oklch(0.48_0.18_25)]">{w.error}</span> : null}
                  <Badge tone={w.status === "COMPLETED" ? "green" : w.status === "FAILED" ? "red" : "yellow"}>{w.status}</Badge>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
