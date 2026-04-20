import { cache } from "react";

export interface InstanceConfig {
    app: string;
    version: string;
    baseUrl: string;
    documentation: string;
    repository: string;
    commit: string;
}

export const getInstanceConfig = cache(async (): Promise<InstanceConfig | null> => {
    try {
        const res = await fetch("http://localhost:8080/api", { cache: "force-cache" });
        if (!res.ok) return null;
        return res.json();
    } catch {
        return null;
    }
});
