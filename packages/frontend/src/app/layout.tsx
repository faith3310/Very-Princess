/**
 * @file layout.tsx
 * @description Root layout for the very-princess Next.js application.
 *
 * This layout wraps every page with:
 *  - Google Fonts (Inter + JetBrains Mono)
 *  - Global Tailwind base styles
 *  - A consistent dark-space background
 *  - SEO meta tags
 */

import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import "./globals.css";

// ── Font Loading ──────────────────────────────────────────────────────────────

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains-mono",
  display: "swap",
});

// ── SEO Metadata ──────────────────────────────────────────────────────────────

export const metadata: Metadata = {
  metadataBase: new URL(
    process.env.NEXT_PUBLIC_APP_URL ?? "http://localhost:3000"
  ),
  title: {
    template: "%s | very-princess",
    default: "very-princess – Stellar Payout Registry",
  },
  description:
    "A decentralized multi-organization payout registry built on Stellar Soroban. Track and claim contributor payouts transparently on-chain.",
  keywords: ["Stellar", "Soroban", "DeFi", "Open Source", "Drips", "Payouts"],
  openGraph: {
    siteName: "very-princess",
    title: "very-princess — Stellar Payout Registry",
    description:
      "A decentralized multi-organization payout registry built on Stellar Soroban. Track and claim contributor payouts transparently on-chain.",
    type: "website",
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    title: "very-princess — Stellar Payout Registry",
    description:
      "A decentralized multi-organization payout registry built on Stellar Soroban. Track and claim contributor payouts transparently on-chain.",
  },
};

// ── Layout ────────────────────────────────────────────────────────────────────

import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Dashboard",
  openGraph: {
    title: "very-princess – Organization Dashboard",
    description: "View organization details and claim contributor payouts on-chain.",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "very-princess – Organization Dashboard",
    description: "View organization details and claim contributor payouts on-chain.",
  },
};

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${inter.variable} ${jetbrainsMono.variable}`}>
      <body className="min-h-screen bg-stellar-blue font-sans text-white antialiased">
        {/* Starfield ambient background */}
        <div
          aria-hidden="true"
          className="pointer-events-none fixed inset-0 bg-hero-pattern"
        />
        {/* Page content */}
        <div className="relative">{children}</div>
      </body>
    </html>
  );
}
