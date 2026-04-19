"use client";

import { useEffect, useRef, useState } from "react";
import { usePathname, useSearchParams } from "next/navigation";

export function NavigationProgress() {
    const pathname = usePathname();
    const searchParams = useSearchParams();
    const [progress, setProgress] = useState(0);
    const [visible, setVisible] = useState(false);
    const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
    const isNavigatingRef = useRef(false);
    const mountedRef = useRef(false);

    function clearTimer() {
        if (intervalRef.current) {
            clearInterval(intervalRef.current);
            intervalRef.current = null;
        }
    }

    function startProgress() {
        isNavigatingRef.current = true;
        setVisible(true);
        setProgress(15);
        clearTimer();
        intervalRef.current = setInterval(() => {
            setProgress((p) => {
                if (p >= 85) {
                    clearTimer();
                    return 85;
                }
                return p + (85 - p) * 0.12;
            });
        }, 120);
    }

    function completeProgress() {
        clearTimer();
        setProgress(100);
        setTimeout(() => {
            setVisible(false);
            setProgress(0);
            isNavigatingRef.current = false;
        }, 300);
    }

    useEffect(() => {
        if (!mountedRef.current) {
            mountedRef.current = true;
            return;
        }
        if (isNavigatingRef.current) {
            completeProgress();
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [pathname, searchParams]);

    useEffect(() => {
        function onLinkClick(e: MouseEvent) {
            const target = e.target as HTMLElement;
            const link = target.closest("a");
            if (
                link?.href?.startsWith(window.location.origin) &&
                !link.target &&
                !link.download &&
                !(e.metaKey || e.ctrlKey || e.shiftKey)
            ) {
                try {
                    const dest = new URL(link.href);
                    if (dest.pathname + dest.search === window.location.pathname + window.location.search) {
                        return;
                    }
                } catch {
                    return;
                }
                startProgress();
            }
        }

        document.addEventListener("click", onLinkClick);
        return () => {
            document.removeEventListener("click", onLinkClick);
            clearTimer();
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    return (
        <div
            className="fixed top-0 left-0 right-0 z-[100] h-0.5 pointer-events-none"
            style={{ opacity: visible ? 1 : 0, transition: "opacity 0.2s" }}
        >
            <div
                className="h-full bg-foreground"
                style={{ width: `${progress}%`, transition: progress === 100 ? "width 0.15s ease-out" : "width 0.1s linear" }}
            />
        </div>
    );
}
