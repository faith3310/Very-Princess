"use client";

import type { AnchorHTMLAttributes, ButtonHTMLAttributes, ReactNode } from "react";

const baseStyles =
  "inline-flex items-center justify-center gap-2 rounded-2xl px-6 py-3 text-sm font-semibold transition duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-stellar-blue disabled:cursor-not-allowed disabled:opacity-50";

const variantStyles: Record<"primary" | "secondary", string> = {
  primary:
    "bg-gradient-to-r from-stellar-purple to-brand-500 text-white shadow-lg shadow-stellar-purple/25 hover:brightness-110",
  secondary:
    "border border-white/15 bg-white/10 text-white/80 hover:bg-white/15 hover:text-white",
};


type GlassButtonProps =
  | (AnchorHTMLAttributes<HTMLAnchorElement> & {
      variant?: "primary" | "secondary";
      href: string;
      children: ReactNode;
      className?: string;
    })
  | (ButtonHTMLAttributes<HTMLButtonElement> & {
      variant?: "primary" | "secondary";
      href?: undefined;
      children: ReactNode;
      className?: string;
    });

type PrimaryButtonProps = Omit<GlassButtonProps, "variant">;
type SecondaryButtonProps = Omit<GlassButtonProps, "variant">;

export function PrimaryButton(props: PrimaryButtonProps) {
  return <GlassButton variant="primary" {...props} />;
}

export function SecondaryButton(props: SecondaryButtonProps) {
  return <GlassButton variant="secondary" {...props} />;
}
