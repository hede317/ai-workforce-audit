import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "HedeYou AI Workforce",
  description: "AI 企业虚拟员工与全域经营自动化平台",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="zh-CN" className="dark">
      <body className="antialiased">{children}</body>
    </html>
  );
}
