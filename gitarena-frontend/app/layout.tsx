import type { Metadata } from "next";
import { Analytics } from "@vercel/analytics/next";
import { Suspense } from "react";
import { GeistSans } from "geist/font/sans";
import { GeistMono } from "geist/font/mono";
import { NavigationProgress } from "@/components/navigation-progress";
import { ClientLayout } from "@/components/client-layout";
import "./globals.css";

export const metadata: Metadata = {
    title: "GitArena - Lightweight Git Platform for Self-Hosting",
    description: "A performant, self-hosted alternative to GitLab and Gitea with built-in VCS, issue tracking, and code review.",
    generator: "GitArena",
    icons: {
        icon: [
            {
                url: "/favicon.svg",
                type: "image/svg+xml",
            },
        ],
    },
};

export default function RootLayout({
    children,
}: Readonly<{
    children: React.ReactNode;
}>) {
    return (
        <html lang="en" className={`${GeistSans.variable} ${GeistMono.variable} bg-background`}>
            <body className="font-sans antialiased">
                <Suspense fallback={null}>
                    <NavigationProgress />
                </Suspense>
                <ClientLayout>{children}</ClientLayout>
                {process.env.NODE_ENV === "production" && <Analytics />}
            </body>
        </html>
    );
}
