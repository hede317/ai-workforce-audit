"use client";

import { useCallback, useEffect, useState } from "react";
import { UserRoundPlus } from "lucide-react";
import { get, post } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner, StatusBadge } from "@/components/ui";

export default function LeadsPage() {
  const t = useT();
  const [items, setItems] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [form, setForm] = useState({ name: "", email: "", phone: "", country: "", business_unit_code: "EDU", source_channel: "website", summary: "" });
  const [openId, setOpenId] = useState<string | null>(null);
  const [events, setEvents] = useState<any[]>([]);
  const [note, setNote] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const d = await get<any>("/api/v1/leads");
      setItems(d.items || []);
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

  async function add() {
    if (!form.name && !form.email) return;
    await post("/api/v1/leads", form);
    setForm({ ...form, name: "", email: "", phone: "", country: "", summary: "" });
    setAdding(false);
    load();
  }

  async function showEvents(id: string) {
    if (openId === id) {
      setOpenId(null);
      return;
    }
    const d = await get<any>(`/api/v1/leads/${id}/events`);
    setEvents(d.items || []);
    setOpenId(id);
  }

  async function addEvent(id: string) {
    if (!note.trim()) return;
    await post(`/api/v1/leads/${id}/events`, { event_type: "followup", note, lead_status: "CONTACTED" });
    setNote("");
    await showEvents(id);
    load();
  }

  return (
    <div className="p-6">
      <h1 className="flex items-center gap-2 text-xl font-bold">
        <UserRoundPlus size={20} /> {t("crm.title")}
      </h1>
      <p className="mb-4 text-xs text-muted-foreground">{t("crm.sub")}</p>
      <ErrorText message={error} />

      <div className="mb-3 flex justify-end">
        <Button size="sm" onClick={() => setAdding(!adding)} variant={adding ? "secondary" : "default"}>
          + {t("crm.add")}
        </Button>
      </div>
      {adding ? (
        <Card className="mb-4 p-4">
          <div className="flex flex-wrap gap-2">
            <input className="h-8 w-28 rounded-md border bg-background px-3 text-xs" placeholder={t("crm.phName")} value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} />
            <input className="h-8 w-44 rounded-md border bg-background px-3 text-xs" placeholder={t("crm.phEmail")} value={form.email} onChange={(e) => setForm({ ...form, email: e.target.value })} />
            <input className="h-8 w-32 rounded-md border bg-background px-3 text-xs" placeholder={t("crm.phPhone")} value={form.phone} onChange={(e) => setForm({ ...form, phone: e.target.value })} />
            <input className="h-8 w-28 rounded-md border bg-background px-3 text-xs" placeholder={t("crm.phCountry")} value={form.country} onChange={(e) => setForm({ ...form, country: e.target.value })} />
            <input className="h-8 w-36 rounded-md border bg-background px-3 text-xs" placeholder={t("crm.phChannel")} value={form.source_channel} onChange={(e) => setForm({ ...form, source_channel: e.target.value })} />
            <input className="h-8 min-w-[200px] flex-1 rounded-md border bg-background px-3 text-xs" placeholder={t("crm.phSummary")} value={form.summary} onChange={(e) => setForm({ ...form, summary: e.target.value })} />
            <Button size="sm" variant="success" onClick={add}>
              {t("c.save")}
            </Button>
          </div>
        </Card>
      ) : null}

      {loading ? (
        <Spinner />
      ) : items.length === 0 ? (
        <Card>
          <Empty text={t("crm.empty")} />
        </Card>
      ) : (
        <Card>
          <div className="divide-y">
            {items.map((l) => (
              <div key={l.id}>
                <div className="flex flex-wrap items-center justify-between gap-3 px-4 py-2.5 hover:bg-accent/40">
                  <button className="min-w-0 flex-1 text-left" onClick={() => showEvents(l.id)}>
                    <div className="truncate text-sm">
                      {l.name || "-"} <span className="text-xs text-muted-foreground">{l.email || ""}</span>
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {l.business_unit_code || "-"} · {l.source_channel || "-"} · {l.country || "-"}
                    </div>
                  </button>
                  <div className="flex items-center gap-1.5">
                    <Badge tone="neutral">{l.lead_status}</Badge>
                    <StatusBadge status={l.lead_status === "WON" ? "Completed" : "NEW"} />
                  </div>
                </div>
                {openId === l.id ? (
                  <div className="bg-[oklch(0.955_0.02_205)] px-4 py-3">
                    <div className="mb-2 text-xs text-muted-foreground">{t("crm.events")}</div>
                    <div className="space-y-1">
                      {events.map((ev) => (
                        <div key={ev.id} className="text-xs">
                          <Badge tone="blue">{ev.event_type}</Badge> <span className="text-muted-foreground">{ev.note}</span>
                        </div>
                      ))}
                      {events.length === 0 ? <div className="text-xs text-muted-foreground">—</div> : null}
                    </div>
                    <div className="mt-2 flex gap-2">
                      <input className="h-8 flex-1 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring" placeholder={t("crm.phNote")} value={note} onChange={(e) => setNote(e.target.value)} />
                      <Button size="sm" variant="secondary" onClick={() => addEvent(l.id)}>
                        {t("crm.followup")}
                      </Button>
                    </div>
                  </div>
                ) : null}
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
