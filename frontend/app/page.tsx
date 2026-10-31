"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Bot } from "lucide-react";
import { post, setToken } from "@/lib/api";
import { useLocale } from "@/lib/i18n";
import { Button, Card, ErrorText } from "@/components/ui";

export default function LoginPage() {
  const router = useRouter();
  const { t } = useLocale();
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const data = await post<{ token: string }>("/api/v1/auth/login", { username, password });
      setToken(data.token);
      router.replace("/dashboard");
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-[radial-gradient(ellipse_at_top,oklch(0.25_0.06_264),oklch(0.16_0_0))]">
      <Card className="w-full max-w-sm p-8">
        <div className="mb-6 flex flex-col items-center gap-2">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-primary text-primary-foreground">
            <Bot size={24} />
          </div>
          <h1 className="text-lg font-bold">{t("login.title")}</h1>
          <p className="text-xs text-muted-foreground">{t("login.sub")}</p>
        </div>
        <form onSubmit={submit} className="space-y-3">
          <div>
            <label className="mb-1 block text-xs text-muted-foreground">{t("login.username")}</label>
            <input
              className="h-9 w-full rounded-md border bg-background px-3 text-sm outline-none focus:ring-[3px] focus:ring-ring"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              autoFocus
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-muted-foreground">{t("login.password")}</label>
            <input
              type="password"
              className="h-9 w-full rounded-md border bg-background px-3 text-sm outline-none focus:ring-[3px] focus:ring-ring"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoFocus
            />
          </div>
          <ErrorText message={error} />
          <Button className="w-full" disabled={loading || !password}>
            {loading ? t("login.loading") : t("login.btn")}
          </Button>
        </form>
      </Card>
    </div>
  );
}
