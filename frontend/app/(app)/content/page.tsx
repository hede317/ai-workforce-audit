"use client";

import { useCallback, useEffect, useState } from "react";
import { FileText, Plus, Sparkles } from "lucide-react";
import { get, post } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner, StatusBadge } from "@/components/ui";

export default function ContentPage() {
  const t = useT();
  const [topics, setTopics] = useState<any[]>([]);
  const [contents, setContents] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const [body, setBody] = useState("");
  const [adding, setAdding] = useState(false);
  const [form, setForm] = useState({ title: "", business_unit_code: "EDU", audience: "" });
  const [bus, setBus] = useState<any[]>([]);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [t, c, b] = await Promise.all([get<any>("/api/v1/topics"), get<any>("/api/v1/contents"), get<any>("/api/v1/business-units")]);
      setTopics(t.items || []);
      setContents(c.items || []);
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

  async function addTopic() {
    if (!form.title) return;
    await post("/api/v1/topics", form);
    setForm({ ...form, title: "", audience: "" });
    setAdding(false);
    load();
  }

  async function draftArticle(topic: any) {
    setBusy(true);
    setError(null);
    try {
      // AUTO 级工具直调；需 AI 配置（NOT_CONFIGURED 会如实报错）
      await post("/api/v1/tools/content.draft_article/invoke", {
        topic: topic.title,
        business_unit_code: topic.business_unit_code,
      });
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }

  async function approve(id: string) {
    try {
      await post(`/api/v1/contents/${id}/approve`);
      load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function view(id: string) {
    if (openId === id) {
      setOpenId(null);
      return;
    }
    try {
      const d = await get<any>(`/api/v1/contents/${id}`);
      setOpenId(id);
      setBody(d.body || t("ct.bodyEmpty"));
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="p-6">
      <h1 className="flex items-center gap-2 text-xl font-bold">
        <FileText size={20} /> {t("ct.title")}
      </h1>
      <p className="mb-4 text-xs text-muted-foreground">{t("ct.sub")}</p>
      <ErrorText message={error} />

      <Card className="mb-4">
        <CardHeader
          title={t("ct.topicPool")}
          desc={t("ct.topicDesc", { n: topics.length })}
          action={
            <Button size="sm" onClick={() => setAdding(!adding)} variant={adding ? "secondary" : "default"}>
              <Plus size={13} /> {t("ct.newTopic")}
            </Button>
          }
        />
        {adding ? (
          <div className="border-b bg-[oklch(0.13_0_0)] px-4 py-3">
            <div className="flex flex-wrap gap-2">
              <input className="h-8 min-w-[260px] flex-1 rounded-md border bg-background px-3 text-xs" placeholder={t("ct.phTitle")} value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} />
              <select className="h-8 rounded-md border bg-background px-2 text-xs" value={form.business_unit_code} onChange={(e) => setForm({ ...form, business_unit_code: e.target.value })}>
                {bus.map((b) => (
                  <option key={b.code} value={b.code}>
                    {b.code}
                  </option>
                ))}
              </select>
              <input className="h-8 w-40 rounded-md border bg-background px-3 text-xs" placeholder={t("ct.phAudience")} value={form.audience} onChange={(e) => setForm({ ...form, audience: e.target.value })} />
              <Button size="sm" variant="success" onClick={addTopic}>
                {t("c.save")}
              </Button>
            </div>
          </div>
        ) : null}
        {loading ? (
          <Spinner />
        ) : topics.length === 0 ? (
          <Empty text={t("ct.emptyTopics")} />
        ) : (
          <div className="divide-y">
            {topics.slice(0, 12).map((t) => (
              <div key={t.id} className="flex items-center justify-between gap-3 px-4 py-2.5 hover:bg-accent/40">
                <div className="min-w-0">
                  <div className="truncate text-sm">{t.title}</div>
                  <div className="text-xs text-muted-foreground">
                    {t.business_unit_code || "-"} · {t.language}
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-1.5">
                  <Badge tone="blue">{t.status}</Badge>
                  <Button size="sm" variant="secondary" disabled={busy} onClick={() => draftArticle(t)}>
                    <Sparkles size={12} /> {t("ct.draft")}
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card>
        <CardHeader title={t("ct.assets")} desc={t("ct.assetsDesc")} />
        {contents.length === 0 ? (
          <Empty text={t("ct.emptyContents")} />
        ) : (
          <div className="divide-y">
            {contents.map((c) => (
              <div key={c.id}>
                <div className="flex items-center justify-between gap-3 px-4 py-2.5 hover:bg-accent/40">
                  <button className="min-w-0 flex-1 text-left" onClick={() => view(c.id)}>
                    <div className="truncate text-sm">{c.title}</div>
                    <div className="text-xs text-muted-foreground">
                      {c.business_unit_code || "-"} · {c.content_type} · {c.language}
                    </div>
                  </button>
                  <div className="flex shrink-0 items-center gap-1.5">
                    {c.status === "DRAFT" ? (
                      <Button size="sm" variant="success" onClick={() => approve(c.id)}>
                        {t("ct.approveBtn")}
                      </Button>
                    ) : null}
                    <StatusBadge status={c.status} />
                  </div>
                </div>
                {openId === c.id ? (
                  <div className="bg-[oklch(0.13_0_0)] px-4 py-3">
                    <pre className="max-h-72 overflow-auto whitespace-pre-wrap font-mono text-[11px] leading-relaxed text-muted-foreground">{body}</pre>
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
