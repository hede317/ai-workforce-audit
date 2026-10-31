"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { Bot, BrainCircuit, FileText, Globe, LayoutDashboard, ListChecks, LogOut, Megaphone, ScrollText, SendHorizonal, ShieldCheck, ShoppingBag, Sparkles, UserRoundPlus } from "lucide-react";
import { clearToken, get } from "@/lib/api";
import { LOCALES, useLocale } from "@/lib/i18n";
import { cn } from "@/lib/cn";

const NAV = [
  { href: "/dashboard", label: "nav.dashboard", icon: LayoutDashboard },
  { href: "/commander", label: "nav.commander", icon: Sparkles },
  { href: "/content", label: "nav.content", icon: FileText },
  { href: "/publishing", label: "nav.publishing", icon: SendHorizonal },
  { href: "/products", label: "nav.products", icon: ShoppingBag },
  { href: "/leads", label: "nav.leads", icon: UserRoundPlus },
  { href: "/campaigns", label: "nav.mkt", icon: Megaphone },
  { href: "/tasks", label: "nav.tasks", icon: ListChecks },
  { href: "/agents", label: "nav.agents", icon: Bot },
  { href: "/approvals", label: "nav.approvals", icon: ShieldCheck },
  { href: "/memory", label: "nav.memory", icon: BrainCircuit },
  { href: "/audit", label: "nav.audit", icon: ScrollText },
];

export default function Shell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const { locale, setLocale, t } = useLocale();
  const [username, setUsername] = useState("");
  const [pending, setPending] = useState(0);

  useEffect(() => {
    if (!localStorage.getItem("awf_token")) {
      router.replace("/");
      return;
    }
    get("/api/v1/auth/me")
      .then((me: any) => setUsername(me.username))
      .catch(() => {
        clearToken();
        router.replace("/");
      });
    get("/api/v1/approvals?status=PENDING")
      .then((d: any) => setPending(d.total || 0))
      .catch(() => {});
  }, [pathname, router]);

  return (
    <div className="flex min-h-screen">
      <aside className="sticky top-0 flex h-screen w-60 shrink-0 flex-col border-r bg-card/50">
        <div className="flex items-center gap-2 px-4 py-4">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
            <Bot size={18} />
          </div>
          <div>
            <div className="text-sm font-semibold leading-tight">AI Workforce</div>
            <div className="text-[10px] text-muted-foreground">{t("shell.brandSub")}</div>
          </div>
        </div>
        <nav className="flex-1 space-y-0.5 px-2">
          {NAV.map(({ href, label, icon: Icon }) => {
            const active = pathname?.startsWith(href);
            return (
              <Link
                key={href}
                href={href}
                className={cn(
                  "flex items-center gap-2.5 rounded-md px-3 py-2 text-sm transition-colors",
                  active ? "bg-accent text-accent-foreground" : "text-muted-foreground hover:bg-accent/50 hover:text-accent-foreground"
                )}
              >
                <Icon size={16} />
                <span className="flex-1">{t(label)}</span>
                {href === "/approvals" && pending > 0 ? (
                  <span className="rounded-full bg-yellow-400/15 px-1.5 py-0.5 text-[10px] font-medium text-[oklch(0.48_0.12_70)]">{pending}</span>
                ) : null}
              </Link>
            );
          })}
        </nav>
        <div className="space-y-2 border-t px-4 py-3">
          <div className="flex items-center gap-1.5">
            <Globe size={13} className="text-muted-foreground" />
            <select
              className="h-7 w-full rounded-md border bg-background px-1.5 text-xs outline-none"
              value={locale}
              onChange={(e) => setLocale(e.target.value as any)}
              title="Language / 语言 / اللغة"
            >
              {LOCALES.map((l) => (
                <option key={l.code} value={l.code}>
                  {l.label}
                </option>
              ))}
            </select>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">{username || "…"}</span>
            <button
              className="text-muted-foreground hover:text-foreground"
              onClick={() => {
                clearToken();
                router.replace("/");
              }}
              title="Logout"
            >
              <LogOut size={15} />
            </button>
          </div>
        </div>
      </aside>
      <main className="min-w-0 flex-1">{children}</main>
    </div>
  );
}
