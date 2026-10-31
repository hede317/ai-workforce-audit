const BASE = process.env.NEXT_PUBLIC_API_BASE || "http://localhost:8600";

export function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return window.localStorage.getItem("awf_token");
}

export function setToken(token: string) {
  window.localStorage.setItem("awf_token", token);
}

export function clearToken() {
  window.localStorage.removeItem("awf_token");
}

export async function api<T = any>(path: string, options: RequestInit = {}): Promise<T> {
  const headers: Record<string, string> = { "Content-Type": "application/json", ...(options.headers as any) };
  const token = getToken();
  if (token) headers["Authorization"] = `Bearer ${token}`;
  const res = await fetch(`${BASE}${path}`, { ...options, headers });
  // 滑动续期：后端剩余<6h时在响应头下发新令牌
  const renewed = res.headers.get("X-Renewed-Token");
  if (renewed) window.localStorage.setItem("awf_token", renewed);
  const payload = await res.json().catch(() => ({ success: false, error: { message: "响应解析失败" } }));
  if (!payload.success) {
    throw new Error(payload.error?.message || `HTTP ${res.status}`);
  }
  return payload.data as T;
}

export const get = <T = any>(path: string) => api<T>(path);
export const post = <T = any>(path: string, body?: any) => api<T>(path, { method: "POST", body: JSON.stringify(body ?? {}) });
export const put = <T = any>(path: string, body?: any) => api<T>(path, { method: "PUT", body: JSON.stringify(body ?? {}) });
