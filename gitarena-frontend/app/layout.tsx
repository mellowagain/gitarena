import type { Metadata } from "next";
import { Analytics } from "@vercel/analytics/next";
import { Suspense } from "react";
import { Host_Grotesk, JetBrains_Mono } from "next/font/google";
import { NavigationProgress } from "@/components/navigation-progress";
import { ClientLayout } from "@/components/client-layout";
import "./globals.css";

const hostGrotesk = Host_Grotesk({
    subsets: ["latin"],
    display: "swap",
    variable: "--font-host-grotesk",
});

const jetbrainsMono = JetBrains_Mono({
    subsets: ["latin"],
    display: "swap",
    variable: "--font-jetbrains-mono",
});

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
        <html lang="en" className={`${hostGrotesk.variable} ${jetbrainsMono.variable} bg-background`}>
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
