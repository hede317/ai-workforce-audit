"use client";

import { useCallback, useEffect, useState } from "react";
import { Plus, ShoppingBag, Sparkles } from "lucide-react";
import { get, post, put } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner } from "@/components/ui";

const STATUS_TONE: Record<string, "neutral" | "yellow" | "green" | "red" | "indigo"> = {
  PENDING: "neutral",
  ANALYZED: "indigo",
  APPROVED: "yellow",
  LISTED: "green",
  REJECTED: "red",
};
const PR_LABEL: Record<string, string> = { PENDING: "PENDING_S" };

export default function ProductsPage() {
  const t = useT();
  const [items, setItems] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [form, setForm] = useState({ title: "", business_unit_code: "ECOM", price: "", source_url: "" });
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const d = await get<any>("/api/v1/products");
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
    if (!form.title) return;
    await post("/api/v1/products", { ...form, price: Number(form.price) || 0 });
    setForm({ ...form, title: "", price: "", source_url: "" });
    setAdding(false);
    load();
  }

  async function score(id: string) {
    setBusy(true);
    setError(null);
    try {
      await post("/api/v1/tools/ecommerce.score_product/invoke", { product_id: id });
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }

  async function setStatus(id: string, status: string) {
    try {
      await put(`/api/v1/products/${id}`, { status });
      load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="p-6">
      <h1 className="flex items-center gap-2 text-xl font-bold">
        <ShoppingBag size={20} /> {t("pr.title")}
      </h1>
      <p className="mb-4 text-xs text-muted-foreground">{t("pr.sub")}</p>
      <ErrorText message={error} />

      <div className="mb-3 flex justify-end">
        <Button size="sm" onClick={() => setAdding(!adding)} variant={adding ? "secondary" : "default"}>
          <Plus size={13} /> {t("pr.add")}
        </Button>
      </div>
      {adding ? (
        <Card className="mb-4 p-4">
          <div className="flex flex-wrap gap-2">
            <input className="h-8 min-w-[240px] flex-1 rounded-md border bg-background px-3 text-xs" placeholder={t("pr.phName")} value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} />
            <input className="h-8 w-28 rounded-md border bg-background px-3 text-xs" placeholder={t("pr.phPrice")} value={form.price} onChange={(e) => setForm({ ...form, price: e.target.value })} />
            <input className="h-8 min-w-[200px] flex-1 rounded-md border bg-background px-3 text-xs" placeholder={t("pr.phUrl")} value={form.source_url} onChange={(e) => setForm({ ...form, source_url: e.target.value })} />
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
          <Empty text={t("pr.empty")} />
        </Card>
      ) : (
        <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {items.map((p) => (
            <Card key={p.id}>
              <CardHeader
                title={p.title}
                desc={`${p.business_unit_code || "-"} · 售价 ¥${p.price}`}
                action={<Badge tone={STATUS_TONE[p.status] || "neutral"}>{t(PR_LABEL[p.status] ?? p.status)}</Badge>}
              />
              <div className="px-4 py-3">
                <div className="grid grid-cols-3 gap-2 text-center">
                  <div className="rounded-md bg-muted/50 py-2">
                    <div className="text-lg font-bold">{p.demand_score || "-"}</div>
                    <div className="text-[10px] text-muted-foreground">{t("pr.demand")}</div>
                  </div>
                  <div className="rounded-md bg-muted/50 py-2">
                    <div className="text-lg font-bold">{p.competition_score || "-"}</div>
                    <div className="text-[10px] text-muted-foreground">{t("pr.comp")}</div>
                  </div>
                  <div className="rounded-md bg-muted/50 py-2">
                    <div className="text-lg font-bold">{p.est_margin_pct ? `${Math.round(p.est_margin_pct)}%` : "-"}</div>
                    <div className="text-[10px] text-muted-foreground">{t("pr.margin")}</div>
                  </div>
                </div>
                {p.decision_note ? <p className="mt-2 line-clamp-3 text-xs leading-relaxed text-muted-foreground">{p.decision_note}</p> : null}
                <div className="mt-3 flex flex-wrap gap-1.5">
                  <Button size="sm" variant="secondary" disabled={busy} onClick={() => score(p.id)}>
                    <Sparkles size={12} /> {t("pr.aiScore")}
                  </Button>
                  {p.status === "ANALYZED" ? (
                    <Button size="sm" variant="success" onClick={() => setStatus(p.id, "APPROVED")}>
                      {t("c.approve")}
                    </Button>
                  ) : null}
                  {p.status === "APPROVED" ? (
                    <Button size="sm" variant="default" onClick={() => setStatus(p.id, "LISTED")}>
                      {t("pr.list")}
                    </Button>
                  ) : null}
                  {["PENDING", "ANALYZED", "APPROVED"].includes(p.status) ? (
                    <Button size="sm" variant="ghost" onClick={() => setStatus(p.id, "REJECTED")}>
                      {t("pr.rejectBtn")}
                    </Button>
                  ) : null}
                  {p.source_url ? (
                    <a href={p.source_url} target="_blank" rel="noreferrer" className="inline-flex h-7 items-center rounded-md px-2.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground">
                      {t("c.link")}
                    </a>
                  ) : null}
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
