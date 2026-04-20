"use client";

import { createContext, useContext } from "react";
import type { InstanceConfig } from "@/lib/instance-config";

const InstanceConfigContext = createContext<InstanceConfig | null>(null);

export function InstanceConfigProvider({ config, children }: { config: InstanceConfig | null; children: React.ReactNode }) {
    return <InstanceConfigContext.Provider value={config}>{children}</InstanceConfigContext.Provider>;
}

export function useInstanceConfig(): InstanceConfig | null {
    return useContext(InstanceConfigContext);
}
