"use client";

import { cn } from "@/lib/cn";
import { useT } from "@/lib/i18n";

export function Card({ className, children }: { className?: string; children: React.ReactNode }) {
  return <div className={cn("rounded-lg border bg-card text-card-foreground shadow-sm", className)}>{children}</div>;
}

export function CardHeader({ title, desc, action }: { title: React.ReactNode; desc?: React.ReactNode; action?: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-3 border-b px-4 py-3">
      <div className="min-w-0">
        <div className="text-sm font-medium truncate">{title}</div>
        {desc ? <div className="text-xs text-muted-foreground mt-0.5">{desc}</div> : null}
      </div>
      {action}
    </div>
  );
}

export function Button({
  children,
  variant = "default",
  size = "default",
  className,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "default" | "secondary" | "ghost" | "destructive" | "success"; size?: "default" | "sm" }) {
  const variants = {
    default: "bg-primary text-primary-foreground hover:opacity-90",
    secondary: "bg-secondary text-secondary-foreground hover:bg-accent",
    ghost: "hover:bg-accent hover:text-accent-foreground",
    destructive: "bg-destructive text-white hover:opacity-90",
    success: "bg-success text-white hover:opacity-90",
  };
  return (
    <button
      className={cn(
        "inline-flex items-center justify-center gap-1.5 rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring disabled:opacity-50 disabled:pointer-events-none",
        size === "sm" ? "h-7 px-2.5 text-xs" : "h-9 px-4",
        variants[variant],
        className
      )}
      {...props}
    >
      {children}
    </button>
  );
}

export function Badge({ children, tone = "neutral", className }: { children: React.ReactNode; tone?: "neutral" | "green" | "cyan" | "yellow" | "red" | "blue" | "indigo" | "violet"; className?: string }) {
  const tones: Record<string, string> = {
    neutral: "bg-muted text-muted-foreground",
    green: "bg-[oklch(0.55_0.14_155/0.14)] text-[oklch(0.42_0.13_155)]",
    cyan: "bg-[oklch(0.55_0.1_210/0.14)] text-[oklch(0.45_0.1_215)]",
    yellow: "bg-[oklch(0.85_0.14_85/0.22)] text-[oklch(0.5_0.12_70)]",
    red: "bg-[oklch(0.55_0.2_25/0.12)] text-[oklch(0.48_0.18_25)]",
    blue: "bg-[oklch(0.58_0.15_245/0.13)] text-[oklch(0.46_0.14_250)]",
    indigo: "bg-[oklch(0.55_0.18_285/0.13)] text-[oklch(0.45_0.16_285)]",
    violet: "bg-[oklch(0.55_0.2_320/0.13)] text-[oklch(0.47_0.17_320)]",
  };
  return <span className={cn("inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium", tones[tone], className)}>{children}</span>;
}

const STATUS_TONE: Record<string, Parameters<typeof Badge>[0]["tone"]> = {
  Completed: "green", Running: "cyan", "Waiting Approval": "yellow", Failed: "red", Cancelled: "neutral", PartialFailed: "yellow",
  Rejected: "red", Created: "blue", Planning: "indigo", Assigned: "indigo", Paused: "yellow",
  APPROVED: "green", DRAFT: "neutral", NEW: "blue", HUMAN_QUEUE: "violet", PUBLISHED: "green",
  ACTIVE: "green", PENDING: "yellow",
};

export function StatusBadge({ status }: { status: string }) {
  const t = useT();
  return <Badge tone={STATUS_TONE[status] || "neutral"}>{t(status)}</Badge>;
}

export function StatusDot({ status }: { status: string }) {
  const map: Record<string, string> = {
    ACTIVE: "bg-success",
    Running: "bg-cyan-400 animate-pulse",
    "Waiting Approval": "bg-yellow-400",
    Failed: "bg-red-400",
    Completed: "bg-green-400",
  };
  return <span className={cn("inline-block h-2 w-2 rounded-full", map[status] || "bg-muted-foreground/50")} />;
}

export function MetricCard({ icon, value, label, description }: { icon?: React.ReactNode; value: React.ReactNode; label: string; description?: string }) {
  return (
    <Card className="p-4">
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">{label}</span>
        {icon ? <span className="text-muted-foreground">{icon}</span> : null}
      </div>
      <div className="mt-1 text-2xl font-bold">{value}</div>
      {description ? <div className="mt-0.5 text-xs text-muted-foreground">{description}</div> : null}
    </Card>
  );
}

export function Empty({ text }: { text?: string }) {
  const t = useT();
  return <div className="px-4 py-10 text-center text-sm text-muted-foreground">{text ?? t("c.none")}</div>;
}

export function ErrorText({ message }: { message?: string | null }) {
  if (!message) return null;
  return <div className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-[oklch(0.45_0.18_25)]">{message}</div>;
}

export function Spinner({ text }: { text?: string }) {
  const t = useT();
  return (
    <div className="flex items-center gap-2 px-4 py-8 text-sm text-muted-foreground">
      <span className="h-3 w-3 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent" />
      {text ?? t("c.loading")}
    </div>
  );
}
