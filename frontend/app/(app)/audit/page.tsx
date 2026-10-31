"use client";

import { useCallback, useEffect, useState } from "react";
import { ScrollText } from "lucide-react";
import { get } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Card, Empty, ErrorText, Spinner } from "@/components/ui";

export default function AuditPage() {
  const [items, setItems] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const t = useT();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const d = await get<any>("/api/v1/audit");
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

  return (
    <div className="p-6">
      <h1 className="flex items-center gap-2 text-xl font-bold">
        <ScrollText size={20} /> {t("audit.title")}
      </h1>
      <p className="mb-4 text-xs text-muted-foreground">{t("audit.sub")}</p>
      <ErrorText message={error} />
      <Card>
        {loading ? (
          <Spinner />
        ) : items.length === 0 ? (
          <Empty text={t("audit.empty")} />
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="border-b bg-accent/20 text-left text-muted-foreground">
                <th className="px-4 py-2 font-medium">{t("audit.time")}</th>
                <th className="px-4 py-2 font-medium">{t("audit.user")}</th>
                <th className="px-4 py-2 font-medium">{t("audit.action")}</th>
                <th className="px-4 py-2 font-medium">{t("audit.entity")}</th>
                <th className="px-4 py-2 font-medium">{t("audit.detail")}</th>
              </tr>
            </thead>
            <tbody>
              {items.map((a) => (
                <tr key={a.id} className="border-b last:border-0 hover:bg-accent/30">
                  <td className="px-4 py-2 font-mono text-muted-foreground">{(a.id || "").slice(0, 8)}</td>
                  <td className="px-4 py-2">{a.user_id || "-"}</td>
                  <td className="px-4 py-2">
                    <span className="rounded bg-primary/10 px-1.5 py-0.5 font-medium text-[oklch(0.45_0.1_205)]">{a.action}</span>
                  </td>
                  <td className="px-4 py-2 text-muted-foreground">
                    {a.entity || "-"}
                    {a.entity_id ? <span className="font-mono"> · {a.entity_id.slice(0, 10)}</span> : null}
                  </td>
                  <td className="max-w-[320px] truncate px-4 py-2 text-muted-foreground">{a.detail ? JSON.stringify(a.detail) : "-"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>
    </div>
  );
}
