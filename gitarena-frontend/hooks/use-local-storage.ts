"use client";

import { useState, useCallback } from "react";

function readFromStorage<T>(key: string, defaultValue: T): T {
    if (typeof window === "undefined") {
        return defaultValue;
    }
    try {
        const stored = localStorage.getItem(key);
        if (stored !== null) {
            return JSON.parse(stored) as T;
        }
    } catch {
        // ignore parse errors
    }
    return defaultValue;
}

export function useLocalStorage<T>(key: string, defaultValue: T): [T, (value: T) => void] {
    const [value, setValue] = useState<T>(() => readFromStorage(key, defaultValue));

    const set = useCallback(
        (newValue: T) => {
            setValue(newValue);
            try {
                localStorage.setItem(key, JSON.stringify(newValue));
            } catch {
                // ignore storage errors
            }
        },
        [key]
    );

    return [value, set];
}
