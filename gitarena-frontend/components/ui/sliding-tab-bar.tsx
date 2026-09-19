"use client";

import { useEffect, useRef } from "react";
import { cn } from "@/lib/utils";

interface TabItem<T extends string> {
    id: T;
    label: string;
    icon?: React.ElementType;
    count?: number | null;
}

interface SlidingTabBarProps<T extends string> {
    items: TabItem<T>[];
    active: T;
    onChange: (id: T) => void;
    vertical?: boolean;
    className?: string;
    tabClassName?: string;
}

export function SlidingTabBar<T extends string>({
    items,
    active,
    onChange,
    vertical = false,
    className,
    tabClassName,
}: SlidingTabBarProps<T>) {
    const barRef = useRef<HTMLDivElement>(null);
    const pillRef = useRef<HTMLSpanElement>(null);

    useEffect(() => {
        const bar = barRef.current;
        if (!bar) {
            return;
        }

        function moveTo(tab: HTMLButtonElement, animate: boolean) {
            const pill = pillRef.current;
            if (!pill) {
                return;
            }
            if (!animate) {
                const prev = pill.style.transition;
                pill.style.transition = "none";
                if (vertical) {
                    pill.style.transform = `translateY(${tab.offsetTop}px)`;
                    pill.style.height = `${tab.offsetHeight}px`;
                } else {
                    pill.style.transform = `translateX(${tab.offsetLeft}px)`;
                    pill.style.width = `${tab.offsetWidth}px`;
                }
                void pill.offsetWidth;
                pill.style.transition = prev;
            } else {
                if (vertical) {
                    pill.style.transform = `translateY(${tab.offsetTop}px)`;
                    pill.style.height = `${tab.offsetHeight}px`;
                } else {
                    pill.style.transform = `translateX(${tab.offsetLeft}px)`;
                    pill.style.width = `${tab.offsetWidth}px`;
                }
            }
        }

        const activeTab = bar.querySelector<HTMLButtonElement>(`[aria-selected="true"]`);
        if (activeTab) {
            moveTo(activeTab, false);
        }

        function onResize() {
            const currentActive = bar!.querySelector<HTMLButtonElement>(`[aria-selected="true"]`);
            if (currentActive) {
                moveTo(currentActive, false);
            }
        }
        window.addEventListener("resize", onResize);
        return () => window.removeEventListener("resize", onResize);
    }, [active, vertical]);

    return (
        <div
            ref={barRef}
            className={cn("relative inline-flex items-center gap-[3px] rounded-lg bg-secondary p-[3px]", vertical && "flex-col", className)}
            role="tablist"
        >
            <span
                ref={pillRef}
                className="absolute top-[3px] left-0 z-0 h-7 w-0 rounded-md bg-accent pointer-events-none transition-[transform,width] duration-200 ease-out"
                aria-hidden="true"
                style={vertical ? { top: 0, left: 3, right: 3, width: "auto", height: 0 } : undefined}
            />
            {items.map((item) => {
                const Icon = item.icon;
                return (
                    <button
                        key={item.id}
                        role="tab"
                        aria-selected={item.id === active ? "true" : "false"}
                        onClick={() => onChange(item.id)}
                        className={cn(
                            "relative z-1 flex h-7 items-center gap-1.5 rounded-md px-3 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground aria-selected:text-foreground",
                            tabClassName
                        )}
                    >
                        {Icon && <Icon className="size-4 shrink-0" />}
                        {item.label}
                        {item.count != null && (
                            <span className="ml-1 rounded-full bg-foreground/10 px-1.5 py-0.5 text-xs tabular-nums">{item.count}</span>
                        )}
                    </button>
                );
            })}
        </div>
    );
}
