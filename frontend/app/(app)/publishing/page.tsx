"use client";

import { useCallback, useEffect, useState } from "react";
import { Package, SendHorizonal } from "lucide-react";
import { get, post } from "@/lib/api";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner, StatusBadge } from "@/components/ui";

export default function PublishingPage() {
  const [tasks, setTasks] = useState<any[]>([]);
  const [contents, setContents] = useState<any[]>([]);
  const [platform, setPlatform] = useState("website");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filling, setFilling] = useState<string | null>(null);
  const [fill, setFill] = useState({ published_url: "", platform_post_id: "" });

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [p, c] = await Promise.all([get<any>("/api/v1/publishing"), get<any>("/api/v1/contents")]);
      setTasks(p.items || []);
      setContents((c.items || []).filter((x: any) => x.status === "APPROVED"));
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

  async function makePackage(contentId: string) {
    try {
      await post(`/api/v1/contents/${contentId}/publish`, { platform });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function submitResult(id: string) {
    try {
      await post(`/api/v1/publishing/${id}/result`, fill);
      setFilling(null);
      setFill({ published_url: "", platform_post_id: "" });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="p-6">
      <h1 className="flex items-center gap-2 text-xl font-bold">
        <SendHorizonal size={20} /> 发布中心
      </h1>
      <p className="mb-4 text-xs text-muted-foreground">未授权平台一律人工上传；发布后回填链接与表现数据，形成内容→发布→线索追溯闭环</p>
      <ErrorText message={error} />

      <Card className="mb-4">
        <CardHeader title={<span className="flex items-center gap-1.5"><Package size={14} /> 已审批待发布内容</span>} desc="内容必须先在内容管理中 APPROVED" action={
          <select className="h-8 rounded-md border bg-background px-2 text-xs" value={platform} onChange={(e) => setPlatform(e.target.value)}>
            {["website", "douyin", "bilibili", "kuaishou", "xiaohongshu", "youtube", "tiktok", "facebook", "instagram", "linkedin"].map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        } />
        {contents.length === 0 ? (
          <Empty text="没有已审批内容" />
        ) : (
          <div className="divide-y">
            {contents.map((c) => (
              <div key={c.id} className="flex items-center justify-between gap-3 px-4 py-2.5">
                <div className="min-w-0">
                  <div className="truncate text-sm">{c.title}</div>
                  <div className="text-xs text-muted-foreground">{c.business_unit_code || "-"} · {c.content_type}</div>
                </div>
                <Button size="sm" variant="secondary" onClick={() => makePackage(c.id)}>
                  生成发布包
                </Button>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card>
        <CardHeader title="发布任务" desc="HUMAN_QUEUE = 人工上传后回填；PUBLISHED = 已回填发布结果" />
        {loading ? (
          <Spinner />
        ) : tasks.length === 0 ? (
          <Empty text="暂无发布任务" />
        ) : (
          <div className="divide-y">
            {tasks.map((t) => (
              <div key={t.id} className="px-4 py-3">
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="truncate text-sm">
                      <Badge tone="blue">{t.platform}</Badge> <span className="ml-1">{(t.payload as any)?.title || t.id.slice(0, 12)}</span>
                    </div>
                    {t.results?.length ? (
                      <div className="mt-0.5 text-xs text-muted-foreground">
                        {t.results.map((r: any, i: number) => (
                          <span key={i} className="mr-2">
                            {r.published_url ? (
                              <a href={r.published_url} target="_blank" rel="noreferrer" className="underline hover:text-foreground">
                                {r.published_url}
                              </a>
                            ) : (
                              r.platform_post_id
                            )}
                            {r.metrics ? ` · ${JSON.stringify(r.metrics)}` : ""}
                          </span>
                        ))}
                      </div>
                    ) : null}
                  </div>
                  <div className="flex shrink-0 items-center gap-1.5">
                    {t.status === "HUMAN_QUEUE" ? (
                      <Button size="sm" variant="secondary" onClick={() => setFilling(filling === t.id ? null : t.id)}>
                        回填结果
                      </Button>
                    ) : null}
                    <StatusBadge status={t.status} />
                  </div>
                </div>
                {filling === t.id ? (
                  <div className="mt-2 flex flex-wrap gap-2">
                    <input
                      className="h-8 min-w-[240px] flex-1 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
                      placeholder="发布链接（https://…）"
                      value={fill.published_url}
                      onChange={(e) => setFill({ ...fill, published_url: e.target.value })}
                    />
                    <input
                      className="h-8 w-44 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
                      placeholder="平台贴子ID（可选）"
                      value={fill.platform_post_id}
                      onChange={(e) => setFill({ ...fill, platform_post_id: e.target.value })}
                    />
                    <Button size="sm" variant="success" onClick={() => submitResult(t.id)}>
                      确认已发布
                    </Button>
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
