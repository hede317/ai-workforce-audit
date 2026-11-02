"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { Package, Play, SendHorizonal, ShieldCheck } from "lucide-react";
import { get, post } from "@/lib/api";
import { useT } from "@/lib/i18n";
import { Badge, Button, Card, CardHeader, Empty, ErrorText, Spinner, StatusBadge } from "@/components/ui";

type PlatformSpec = { key: string; label: string; region: "DOMESTIC" | "OVERSEAS"; needs_proxy: boolean };

const FALLBACK_DOMESTIC: PlatformSpec[] = [
  { key: "douyin", label: "抖音", region: "DOMESTIC", needs_proxy: false },
  { key: "bilibili", label: "B站", region: "DOMESTIC", needs_proxy: false },
  { key: "kuaishou", label: "快手", region: "DOMESTIC", needs_proxy: false },
  { key: "xiaohongshu", label: "小红书", region: "DOMESTIC", needs_proxy: false },
  { key: "tencent", label: "视频号", region: "DOMESTIC", needs_proxy: false },
];
const FALLBACK_OVERSEAS: PlatformSpec[] = [
  { key: "youtube", label: "YouTube", region: "OVERSEAS", needs_proxy: true },
  { key: "tiktok", label: "TikTok", region: "OVERSEAS", needs_proxy: true },
];

export default function PublishingPage() {
  const t = useT();
  const [tasks, setTasks] = useState<any[]>([]);
  const [contents, setContents] = useState<any[]>([]);
  const [byRegion, setByRegion] = useState<{ domestic: number; overseas: number }>({ domestic: 0, overseas: 0 });
  const [platform, setPlatform] = useState("douyin");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filling, setFilling] = useState<string | null>(null);
  const [fill, setFill] = useState({ published_url: "", platform_post_id: "" });

  // 自动发布状态
  const [catalog, setCatalog] = useState<{ domestic: PlatformSpec[]; overseas: PlatformSpec[] }>({
    domestic: FALLBACK_DOMESTIC,
    overseas: FALLBACK_OVERSEAS,
  });
  const [autoFile, setAutoFile] = useState("");
  const [autoTitle, setAutoTitle] = useState("");
  const [autoDesc, setAutoDesc] = useState("");
  const [autoTags, setAutoTags] = useState("");
  const [autoAccount, setAutoAccount] = useState("creator");
  const [autoPicked, setAutoPicked] = useState<string[]>(["douyin"]);
  const [autoRunning, setAutoRunning] = useState(false);
  const [autoResult, setAutoResult] = useState<any>(null);
  const [accStatus, setAccStatus] = useState<Record<string, string>>({});

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [p, c] = await Promise.all([get<any>("/api/v1/publishing"), get<any>("/api/v1/contents")]);
      setTasks(p.items || []);
      setByRegion(p.by_region || { domestic: 0, overseas: 0 });
      setContents((c.items || []).filter((x: any) => x.status === "APPROVED"));
      setError(null);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadCatalog = useCallback(async () => {
    try {
      const r = await post<any>("/api/v1/tools/publish.platforms/invoke", {});
      const data = r?.data ?? r;
      if (data?.domestic?.length) {
        setCatalog({ domestic: data.domestic, overseas: data.overseas || [] });
      }
    } catch {
      // 后端没开自动发布时保持内置清单，不阻断页面
    }
  }, []);

  useEffect(() => {
    load();
    loadCatalog();
  }, [load, loadCatalog]);

  const allPlatforms = useMemo(() => [...catalog.domestic, ...catalog.overseas], [catalog]);
  const labelOf = useCallback(
    (key: string) => allPlatforms.find((p) => p.key === key)?.label || key,
    [allPlatforms],
  );

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

  function togglePick(key: string) {
    setAutoPicked((prev) => (prev.includes(key) ? prev.filter((k) => k !== key) : [...prev, key]));
  }

  async function checkAccount(key: string) {
    setAccStatus((s) => ({ ...s, [key]: "..." }));
    try {
      const r = await post<any>("/api/v1/tools/publish.check_account/invoke", { platform: key, account: autoAccount });
      const data = r?.data ?? r;
      setAccStatus((s) => ({ ...s, [key]: String(data?.status || "UNKNOWN") }));
    } catch (e: any) {
      setAccStatus((s) => ({ ...s, [key]: "ERROR" }));
      setError(e.message);
    }
  }

  async function runAuto() {
    if (!autoFile.trim() || !autoTitle.trim() || autoPicked.length === 0) {
      setError(t("pub.autoDesc"));
      return;
    }
    setAutoRunning(true);
    setError(null);
    setAutoResult(null);
    try {
      const body: any = {
        file: autoFile.trim(),
        title: autoTitle.trim(),
        account: autoAccount.trim() || "creator",
        desc: autoDesc.trim(),
        tags: autoTags
          .split(/[,，\s]+/)
          .map((s) => s.trim())
          .filter(Boolean),
      };
      const tool = autoPicked.length === 1 ? "publish.auto" : "publish.auto_many";
      if (autoPicked.length === 1) body.platform = autoPicked[0];
      else body.platforms = autoPicked;
      const r = await post<any>(`/api/v1/tools/${tool}/invoke`, body);
      setAutoResult(r?.data ?? r);
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setAutoRunning(false);
    }
  }

  function accTone(status?: string) {
    if (!status) return "neutral" as const;
    if (status === "VALID") return "green" as const;
    if (status === "INVALID") return "red" as const;
    return "yellow" as const;
  }

  return (
    <div className="p-6">
      <h1 className="flex items-center gap-2 text-xl font-bold">
        <SendHorizonal size={20} /> {t("pub.title")}
      </h1>
      <p className="mb-4 text-xs text-muted-foreground">{t("pub.sub")}</p>
      <ErrorText message={error} />

      {/* 自动发布：国内 / 海外分区 */}
      <Card className="mb-4">
        <CardHeader
          title={
            <span className="flex items-center gap-1.5">
              <ShieldCheck size={14} /> {t("pub.auto")}
            </span>
          }
          desc={t("pub.autoDesc")}
        />
        <div className="space-y-3 px-4 pb-4">
          <div className="grid gap-2 md:grid-cols-2">
            <input
              className="h-8 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
              placeholder={t("pub.pickFile")}
              value={autoFile}
              onChange={(e) => setAutoFile(e.target.value)}
            />
            <input
              className="h-8 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
              placeholder={t("pub.auto") + " · title"}
              value={autoTitle}
              onChange={(e) => setAutoTitle(e.target.value)}
            />
            <input
              className="h-8 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
              placeholder="desc"
              value={autoDesc}
              onChange={(e) => setAutoDesc(e.target.value)}
            />
            <input
              className="h-8 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
              placeholder="tags,逗号分隔"
              value={autoTags}
              onChange={(e) => setAutoTags(e.target.value)}
            />
            <input
              className="h-8 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
              placeholder="account"
              value={autoAccount}
              onChange={(e) => setAutoAccount(e.target.value)}
            />
          </div>

          {(["domestic", "overseas"] as const).map((region) => (
            <div key={region}>
              <div className="mb-1 text-xs font-medium text-muted-foreground">
                {region === "domestic" ? t("pub.domestic") : t("pub.overseas")}
              </div>
              <div className="flex flex-wrap gap-1.5">
                {catalog[region].map((p) => (
                  <div key={p.key} className="flex items-center gap-1">
                    <button
                      onClick={() => togglePick(p.key)}
                      className={`rounded-md border px-2.5 py-1 text-xs transition ${
                        autoPicked.includes(p.key)
                          ? "border-primary bg-primary/10 text-foreground"
                          : "bg-background text-muted-foreground hover:text-foreground"
                      }`}
                    >
                      {p.label}
                    </button>
                    <button
                      onClick={() => checkAccount(p.key)}
                      title={t("pub.checkAcc")}
                      className="rounded border px-1.5 py-0.5 text-[10px] text-muted-foreground hover:text-foreground"
                    >
                      {accStatus[p.key] ? (
                        <Badge tone={accTone(accStatus[p.key])}>{accStatus[p.key]}</Badge>
                      ) : (
                        "?"
                      )}
                    </button>
                  </div>
                ))}
              </div>
            </div>
          ))}

          <div className="flex items-center gap-2">
            <Button size="sm" variant="success" onClick={runAuto} disabled={autoRunning}>
              <Play size={13} /> {autoRunning ? t("pub.autoRunning") : t("pub.autoRun")}
            </Button>
            <span className="text-xs text-muted-foreground">
              {autoPicked.length} / {allPlatforms.length}
            </span>
          </div>

          {autoResult ? (
            <div className="rounded-md border bg-muted/30 p-2 text-xs">
              <div className="mb-1 font-medium">{autoResult.output}</div>
              {autoResult.results ? (
                <div className="space-y-0.5">
                  {autoResult.results.map((r: any) => (
                    <div key={r.publishing_task_id} className="flex items-center gap-2">
                      <Badge tone={r.ok ? "green" : "red"}>{labelOf(r.platform)}</Badge>
                      <span className="text-muted-foreground">
                        {r.region} · {r.ok ? "OK" : r.timed_out ? "TIMEOUT" : `EXIT_${r.exit_code}`}
                      </span>
                    </div>
                  ))}
                </div>
              ) : autoResult.log_tail ? (
                <pre className="max-h-40 overflow-auto whitespace-pre-wrap text-[10px] text-muted-foreground">
                  {autoResult.log_tail}
                </pre>
              ) : null}
            </div>
          ) : null}
        </div>
      </Card>

      <Card className="mb-4">
        <CardHeader
          title={
            <span className="flex items-center gap-1.5">
              <Package size={14} /> {t("pub.approvedList")}
            </span>
          }
          desc={t("pub.approvedDesc")}
          action={
            <select
              className="h-8 rounded-md border bg-background px-2 text-xs"
              value={platform}
              onChange={(e) => setPlatform(e.target.value)}
            >
              {allPlatforms.map((p) => (
                <option key={p.key} value={p.key}>
                  {p.label}
                </option>
              ))}
              <option value="website">website</option>
            </select>
          }
        />
        {contents.length === 0 ? (
          <Empty text={t("pub.noApproved")} />
        ) : (
          <div className="divide-y">
            {contents.map((c) => (
              <div key={c.id} className="flex items-center justify-between gap-3 px-4 py-2.5">
                <div className="min-w-0">
                  <div className="truncate text-sm">{c.title}</div>
                  <div className="text-xs text-muted-foreground">
                    {c.business_unit_code || "-"} · {c.content_type}
                  </div>
                </div>
                <Button size="sm" variant="secondary" onClick={() => makePackage(c.id)}>
                  {t("pub.makePkg")}
                </Button>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card>
        <CardHeader
          title={t("pub.tasks")}
          desc={t("pub.tasksDesc")}
          action={
            <span className="text-xs text-muted-foreground">
              {t("pub.byRegion")
                .replace("{a}", String(byRegion.domestic))
                .replace("{b}", String(byRegion.overseas))}
            </span>
          }
        />
        {loading ? (
          <Spinner />
        ) : tasks.length === 0 ? (
          <Empty text={t("pub.empty")} />
        ) : (
          <div className="divide-y">
            {tasks.map((task) => (
              <div key={task.id} className="px-4 py-3">
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="truncate text-sm">
                      <Badge tone={task.region === "OVERSEAS" ? "yellow" : "blue"}>{labelOf(task.platform)}</Badge>{" "}
                      <span className="ms-1">{(task.payload as any)?.title || task.id.slice(0, 12)}</span>
                    </div>
                    <div className="mt-0.5 flex items-center gap-1.5 text-[11px] text-muted-foreground">
                      <span>{task.mode === "AUTO" ? t("pub.autoMode") : t("pub.humanMode")}</span>
                      {task.account ? <span>· {task.account}</span> : null}
                    </div>
                    {task.results?.length ? (
                      <div className="mt-0.5 text-xs text-muted-foreground">
                        {task.results.map((r: any, i: number) => (
                          <span key={i} className="me-2">
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
                    {task.status === "FAILED" && task.exit_log_tail ? (
                      <details className="mt-1">
                        <summary className="cursor-pointer text-[11px] text-muted-foreground">{t("pub.logTail")}</summary>
                        <pre className="mt-1 max-h-32 overflow-auto whitespace-pre-wrap text-[10px] text-muted-foreground">
                          {task.exit_log_tail}
                        </pre>
                      </details>
                    ) : null}
                  </div>
                  <div className="flex shrink-0 items-center gap-1.5">
                    {task.status === "HUMAN_QUEUE" ? (
                      <Button size="sm" variant="secondary" onClick={() => setFilling(filling === task.id ? null : task.id)}>
                        {t("pub.fill")}
                      </Button>
                    ) : null}
                    <StatusBadge status={task.status} />
                  </div>
                </div>
                {filling === task.id ? (
                  <div className="mt-2 flex flex-wrap gap-2">
                    <input
                      className="h-8 min-w-[240px] flex-1 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
                      placeholder={t("pub.urlPh")}
                      value={fill.published_url}
                      onChange={(e) => setFill({ ...fill, published_url: e.target.value })}
                    />
                    <input
                      className="h-8 w-44 rounded-md border bg-background px-3 text-xs outline-none focus:ring-[3px] focus:ring-ring"
                      placeholder={t("pub.postIdPh")}
                      value={fill.platform_post_id}
                      onChange={(e) => setFill({ ...fill, platform_post_id: e.target.value })}
                    />
                    <Button size="sm" variant="success" onClick={() => submitResult(task.id)}>
                      {t("pub.confirmPub")}
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
