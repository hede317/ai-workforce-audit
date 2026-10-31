"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { Bot, BrainCircuit, FileText, LayoutDashboard, ListChecks, LogOut, SendHorizonal, ShieldCheck, ShoppingBag, Sparkles } from "lucide-react";
import { clearToken, get } from "@/lib/api";
import { cn } from "@/lib/cn";

const NAV = [
  { href: "/dashboard", label: "总览", icon: LayoutDashboard },
  { href: "/commander", label: "AI Commander", icon: Sparkles },
  { href: "/content", label: "内容中心", icon: FileText },
  { href: "/publishing", label: "发布中心", icon: SendHorizonal },
  { href: "/products", label: "选品中心", icon: ShoppingBag },
  { href: "/tasks", label: "任务中心", icon: ListChecks },
  { href: "/agents", label: "Agent 团队", icon: Bot },
  { href: "/approvals", label: "审批中心", icon: ShieldCheck },
  { href: "/memory", label: "记忆中心", icon: BrainCircuit },
];

export default function Shell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
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
            <div className="text-[10px] text-muted-foreground">HedeYou 虚拟员工平台</div>
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
                <span className="flex-1">{label}</span>
                {href === "/approvals" && pending > 0 ? (
                  <span className="rounded-full bg-yellow-400/15 px-1.5 py-0.5 text-[10px] font-medium text-[oklch(0.85_0.13_85)]">{pending}</span>
                ) : null}
              </Link>
            );
          })}
        </nav>
        <div className="border-t px-4 py-3">
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">{username || "…"}</span>
            <button
              className="text-muted-foreground hover:text-foreground"
              onClick={() => {
                clearToken();
                router.replace("/");
              }}
              title="退出登录"
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
