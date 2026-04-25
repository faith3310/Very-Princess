/**
 * @file payouts/page.tsx
 * @description Maintainer dashboard for viewing and claiming payouts.
 */

"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import useSWR from "swr";
import { WalletButton } from "@/components/WalletButton";
import { useSSEWithSWR } from "@/hooks/useSSE";
import { useClaimPayout } from "@/hooks/useClaimPayout";
import { useWallet } from "@/hooks/useWallet";

interface PendingPayout {
  orgId: string;
  amountStroops: string;
  amountXlm: string;
  orgName?: string;
}

export default function PayoutsPage() {
  const { isConnected, publicKey } = useWallet();
  const { claimPayout, isClaiming } = useClaimPayout();

  // Enable SSE for real-time updates
  useSSEWithSWR();

  // Fetch pending payouts for the connected wallet
  const { data: payouts, error, isLoading, mutate } = useSWR(
    isConnected && publicKey ? [`/api/v1/contract/maintainer/${publicKey}`] : null,
    async ([url]) => {
      const response = await fetch(`${process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3001"}${url}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch payouts: ${response.statusText}`);
      }
      const data = await response.json();
      
      // Transform data to include XLM amount
      return data.map((payout: any) => ({
        ...payout,
        amountXlm: (Number(payout.amountStroops) / 10_000_000).toFixed(2),
      }));
    },
    {
      revalidateOnFocus: false,
      revalidateOnReconnect: true,
    }
  );

  const handleClaimPayout = async (orgId: string) => {
    try {
      await claimPayout(orgId);
      // Refresh the payouts list after successful claim
      await mutate();
    } catch (error) {
      console.error("Failed to claim payout:", error);
    }
  };

  if (!isConnected) {
    return (
      <div className="flex min-h-screen flex-col">
        {/* Navigation */}
        <header className="sticky top-0 z-50 border-b border-white/[0.06] bg-stellar-blue/80 backdrop-blur-xl">
          <nav className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
            <div className="flex items-center gap-4">
              <Link href="/" className="flex items-center gap-2 text-white/60 transition-colors hover:text-white">
                <span className="text-sm font-bold">VP</span>
              </Link>
              <span className="text-white/20">/</span>
              <Link href="/dashboard" className="text-sm text-white/60 hover:text-white">Dashboard</Link>
              <span className="text-white/20">/</span>
              <h1 className="text-sm font-semibold text-white">Payouts</h1>
            </div>
            <WalletButton />
          </nav>
        </header>

        <main className="mx-auto w-full max-w-4xl flex-1 px-6 py-20">
          <div className="text-center">
            <h2 className="text-2xl font-bold text-white mb-4">Connect Your Wallet</h2>
            <p className="text-white/60 mb-8">
              Please connect your Freighter wallet to view and claim your pending payouts.
            </p>
            <WalletButton />
          </div>
        </main>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen flex-col">
      {/* Navigation */}
      <header className="sticky top-0 z-50 border-b border-white/[0.06] bg-stellar-blue/80 backdrop-blur-xl">
        <nav className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
          <div className="flex items-center gap-4">
            <Link href="/" className="flex items-center gap-2 text-white/60 transition-colors hover:text-white">
              <span className="text-sm font-bold">VP</span>
            </Link>
            <span className="text-white/20">/</span>
            <Link href="/dashboard" className="text-sm text-white/60 hover:text-white">Dashboard</Link>
            <span className="text-white/20">/</span>
            <h1 className="text-sm font-semibold text-white">Payouts</h1>
          </div>
          <WalletButton />
        </nav>
      </header>

      <main className="mx-auto w-full max-w-4xl flex-1 px-6 py-10">
        <div className="mb-8">
          <h2 className="text-3xl font-bold text-white mb-2">Your Payouts</h2>
          <p className="text-white/50">
            Manage and claim your pending payouts from organizations you contribute to.
          </p>
        </div>

        {error && (
          <div className="mb-8 rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-400">
            {error.message || "Failed to load payouts"}
          </div>
        )}

        {isLoading ? (
          <div className="space-y-4">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="glass-card animate-pulse">
                <div className="flex items-center justify-between p-6">
                  <div className="space-y-2">
                    <div className="h-4 w-32 bg-white/10 rounded"></div>
                    <div className="h-3 w-24 bg-white/5 rounded"></div>
                  </div>
                  <div className="h-8 w-20 bg-white/10 rounded"></div>
                </div>
              </div>
            ))}
          </div>
        ) : payouts && payouts.length > 0 ? (
          <div className="space-y-4">
            {payouts.map((payout: PendingPayout, index: number) => (
              <div key={`${payout.orgId}-${index}`} className="glass-card">
                <div className="flex items-center justify-between p-6">
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <div className="h-10 w-10 rounded-lg bg-stellar-purple/20 flex items-center justify-center">
                        <span className="text-lg">🏛️</span>
                      </div>
                      <div>
                        <h3 className="font-semibold text-white">
                          {payout.orgName || payout.orgId}
                        </h3>
                        <p className="text-sm text-white/40 font-mono">{payout.orgId}</p>
                      </div>
                    </div>
                    <div className="flex items-center gap-4 text-sm">
                      <div className="flex items-center gap-2 text-white/60">
                        <span className="h-1.5 w-1.5 rounded-full bg-stellar-teal"></span>
                        Claimable: <span className="font-mono">{payout.amountXlm} XLM</span>
                      </div>
                    </div>
                  </div>
                  <button
                    onClick={() => handleClaimPayout(payout.orgId)}
                    disabled={isClaiming}
                    className="rounded-lg bg-gradient-to-r from-stellar-purple to-brand-500 px-6 py-2.5 text-sm font-semibold text-white shadow-lg shadow-stellar-purple/20 transition-all hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {isClaiming ? "Claiming..." : "Claim"}
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-20">
            <div className="mb-4">
              <div className="h-16 w-16 rounded-full bg-white/5 flex items-center justify-center mx-auto">
                <span className="text-2xl">💰</span>
              </div>
            </div>
            <h3 className="text-xl font-semibold text-white mb-2">No Pending Payouts</h3>
            <p className="text-white/50">
              You don't have any pending payouts at the moment. Check back later or contact organizations you contribute to.
            </p>
          </div>
        )}
      </main>
    </div>
  );
}
