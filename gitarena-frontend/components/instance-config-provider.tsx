"use client";

import useSWR from "swr";
import type { InstanceConfig } from "@/lib/instance-config";

export function useInstanceConfig(): InstanceConfig | null {
    const { data } = useSWR<InstanceConfig>("/api");
    return data ?? null;
}
