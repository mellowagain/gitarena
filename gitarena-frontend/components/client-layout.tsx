"use client";

import { useRouter } from "next/navigation";
import { useEffect, useRef } from "react";
import { SWRConfig } from "swr";
import { InstanceConfigProvider } from "@/components/instance-config-provider";
import type { InstanceConfig } from "@/lib/instance-config";

const fetcher = (url: string) =>
    fetch(url).then((res) => {
        if (!res.ok) {
            throw new Error(res.statusText);
        }
        return res.json();
    });

export function ClientLayout({ instanceConfig, children }: { instanceConfig: InstanceConfig | null; children: React.ReactNode }) {
    const router = useRouter();
    const lastPathRef = useRef("");

    useEffect(() => {
        lastPathRef.current = window.location.pathname + window.location.search;
    });

    useEffect(() => {
        function handlePopState() {
            const browserPath = window.location.pathname + window.location.search;
            if (browserPath !== lastPathRef.current) {
                lastPathRef.current = browserPath;
                router.replace(browserPath);
            }
        }

        window.addEventListener("popstate", handlePopState);
        return () => window.removeEventListener("popstate", handlePopState);
    }, [router]);

    return (
        <InstanceConfigProvider config={instanceConfig}>
            <SWRConfig value={{ fetcher }}>{children}</SWRConfig>
        </InstanceConfigProvider>
    );
}
