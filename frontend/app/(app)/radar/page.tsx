"use client";

import { useCallback, useEffect, useState } from "react";
import { Radar, ScanSearch, Trash2 } from "lucide-react";
import { get, post } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner } from "@/components/ui";

const SIGNAL_TONE: Record<string, "neutral" | "green" | "blue" | "violet"> = {
  NEW: "blue",
  CONVERTED: "green",
  DISMISSED: "neutral",
};

export default function RadarPage() {
  const t = useT();
  const [kws, setKws] = useState<any[]>([]);
  const [signals, setSignals] = useState<any[]>([]);
  const [bus, setBus] = useState<any[]>([]);
  const [bu, setBu] = useState("ERP");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [kwForm, setKwForm] = useState({ keyword: "", goal_text: "" });

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [k, s, b] = await Promise.all([
        get<any>("/api/v1/listening/keywords"),
        get<any>("/api/v1/listening/signals"),
        get<any>("/api/v1/business-units"),
      ]);
      setKws(k.items || []);
      setSignals(s.items || []);
      setBus(b.items || []);
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

  async function addKeyword() {
    if (!kwForm.keyword.trim()) return;
    try {
      await post("/api/v1/listening/keywords", { ...kwForm, business_unit_code: bu });
      setKwForm({ keyword: "", goal_text: "" });
      load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function removeKeyword(id: string) {
    try {
      await fetch(`${process.env.NEXT_PUBLIC_API_BASE || "http://localhost:8600"}/api/v1/listening/keywords/${id}`, {
        method: "DELETE",
        headers: { Authorization: `Bearer ${localStorage.getItem("awf_token")}` },
      });
      load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function scan() {
    setScanning(true);
    setError(null);
    setNotice(null);
    try {
      const r = await post<any>("/api/v1/tools/listen.scan_keywords/invoke", { business_unit_code: bu || undefined });
      setNotice(r.output || t("rad.scanDone"));
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setScanning(false);
    }
  }

  async function convert(id: string, kind: "topic" | "lead" | "script") {
    setError(null);
    setNotice(null);
    try {
      if (kind === "script") {
        const sig = signals.find((s) => s.id === id);
        await post("/api/v1/tools/content.video_script/invoke", {
          topic: `${sig?.title || ""}（素材：${sig?.summary || ""}）`,
          business_unit_code: sig?.business_unit_code,
        });
        setNotice(t("rad.scriptDone"));
      } else {
        await post(`/api/v1/listening/signals/${id}/to-${kind}`);
      }
      load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="p-6">
      <h1 className="flex items-center gap-2 text-xl font-bold">
        <Radar size={20} /> {t("rad.title")}
      </h1>
      <p className="mb-4 text-xs text-muted-foreground">{t("rad.sub")}</p>
      <ErrorText message={error} />
      {notice ? <div className="mb-3 rounded-md border border-success/40 bg-[oklch(0.55_0.14_155/0.08)] px-3 py-2 text-xs text-[oklch(0.42_0.13_155)]">{notice}</div> : null}

      <div className="grid gap-4 lg:grid-cols-3">
        <Card>
          <CardHeader title={t("rad.kw")} desc={t("rad.kwDesc")} />
          <div className="border-b px-4 py-3">
            <div className="flex gap-1.5">
              <input className="h-8 min-w-0 flex-1 rounded-md border bg-background px-2 text-xs" placeholder={t("rad.kwPh")} value={kwForm.keyword} onChange={(e) => setKwForm({ ...kwForm, keyword: e.target.value })} />
              <Button size="sm" onClick={addKeyword}>
                + {t("c.save")}
              </Button>
            </div>
            <input className="mt-1.5 h-8 w-full rounded-md border bg-background px-2 text-xs" placeholder={t("rad.goalPh")} value={kwForm.goal_text} onChange={(e) => setKwForm({ ...kwForm, goal_text: e.target.value })} />
          </div>
          <div className="max-h-72 divide-y overflow-auto">
            {kws.length === 0 ? <Empty text={t("rad.kwEmpty")} /> : null}
            {kws.map((k) => (
              <div key={k.id} className="flex items-center justify-between gap-2 px-4 py-2">
                <div className="min-w-0">
                  <div className="truncate text-xs font-medium">{k.keyword}</div>
                  <div className="truncate text-[10px] text-muted-foreground">
                    {k.business_unit_code || "-"} · {k.goal_text || "-"}
                  </div>
                </div>
                <button className="text-muted-foreground hover:text-destructive" onClick={() => removeKeyword(k.id)}>
                  <Trash2 size={13} />
                </button>
              </div>
            ))}
          </div>
        </Card>

        <Card className="lg:col-span-2">
          <CardHeader
            title={t("rad.signals")}
            desc={t("rad.signalsDesc")}
            action={
              <div className="flex items-center gap-1.5">
                <select className="h-8 rounded-md border bg-background px-2 text-xs" value={bu} onChange={(e) => setBu(e.target.value)}>
                  {bus.map((b) => (
                    <option key={b.code} value={b.code}>
                      {b.code}
                    </option>
                  ))}
                </select>
                <Button size="sm" onClick={scan} disabled={scanning}>
                  <ScanSearch size={13} /> {scanning ? t("rad.scanning") : t("rad.scan")}
                </Button>
              </div>
            }
          />
          {loading ? (
            <Spinner />
          ) : signals.length === 0 ? (
            <Empty text={t("rad.empty")} />
          ) : (
            <div className="max-h-[560px] divide-y overflow-auto">
              {signals.map((s) => (
                <div key={s.id} className="px-4 py-3">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="truncate text-sm">
                        <Badge tone={s.score >= 8 ? "red" : s.score >= 6 ? "yellow" : "neutral"}>{s.score}</Badge>{" "}
                        {s.url ? (
                          <a href={s.url} target="_blank" rel="noreferrer" className="underline hover:text-primary">
                            {s.title}
                          </a>
                        ) : (
                          s.title
                        )}
                      </div>
                      <div className="text-[10px] text-muted-foreground">
                        {s.platform} · {s.keyword || "-"} · {s.business_unit_code || "-"} · {(s.created_at || "").replace("T", " ").slice(0, 16)}
                      </div>
                    </div>
                    <div className="flex shrink-0 items-center gap-1">
                      {s.status === "NEW" ? (
                        <>
                          <Button size="sm" variant="secondary" onClick={() => convert(s.id, "topic")}>
                            {t("rad.toTopic")}
                          </Button>
                          <Button size="sm" variant="ghost" onClick={() => convert(s.id, "lead")}>
                            {t("rad.toLead")}
                          </Button>
                          <Button size="sm" variant="ghost" onClick={() => convert(s.id, "script")}>
                            {t("rad.toScript")}
                          </Button>
                        </>
                      ) : (
                        <Badge tone={SIGNAL_TONE[s.status] || "neutral"}>{s.status}</Badge>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}
