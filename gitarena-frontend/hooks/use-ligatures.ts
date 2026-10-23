"use client";

import { useEffect } from "react";
import { useLocalStorage } from "@/hooks/use-local-storage";

export function useLigatures(): [boolean, (value: boolean) => void] {
    const [ligatures, setLigatures] = useLocalStorage<boolean>("gitarena:ligatures", false);

    useEffect(() => {
        document.documentElement.classList.toggle("ligatures", ligatures);
    }, [ligatures]);

    return [ligatures, setLigatures];
}
