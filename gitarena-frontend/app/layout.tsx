import type { Metadata } from "next";
import { Analytics } from "@vercel/analytics/next";
import { Suspense } from "react";
import { NavigationProgress } from "@/components/navigation-progress";
import { ClientLayout } from "@/components/client-layout";
import { getInstanceConfig } from "@/lib/instance-config";
import "./globals.css";

export const metadata: Metadata = {
    title: "GitArena - Lightweight Git Platform for Self-Hosting",
    description: "A performant, self-hosted alternative to GitLab and Gitea with built-in VCS, issue tracking, and code review.",
    generator: "v0.app",
    icons: {
        icon: [
            {
                url: "/icon-light-32x32.png",
                media: "(prefers-color-scheme: light)",
            },
            {
                url: "/icon-dark-32x32.png",
                media: "(prefers-color-scheme: dark)",
            },
            {
                url: "/icon.svg",
                type: "image/svg+xml",
            },
        ],
        apple: "/apple-icon.png",
    },
};

export default async function RootLayout({
    children,
}: Readonly<{
    children: React.ReactNode;
}>) {
    const instanceConfig = await getInstanceConfig();

    return (
        <html lang="en" className="bg-background">
            <body className="font-sans antialiased">
                <Suspense fallback={null}>
                    <NavigationProgress />
                </Suspense>
                <ClientLayout instanceConfig={instanceConfig}>{children}</ClientLayout>
                {process.env.NODE_ENV === "production" && <Analytics />}
            </body>
        </html>
    );
}
