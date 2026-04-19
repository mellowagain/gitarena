"use client";

import { useRouter } from "next/navigation";
import { useEffect, useRef } from "react";

export function ClientLayout({ children }: { children: React.ReactNode }) {
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

    return <>{children}</>;
}
