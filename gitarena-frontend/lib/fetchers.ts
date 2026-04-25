interface ApiError {
    error: string;
}

/**
 * Generic JSON GET fetcher. Used as the global SWR fetcher via SWRConfig
 * and can be imported explicitly when needed.
 */
export const jsonFetcher = (url: string) =>
    fetch(url).then((res) => {
        if (!res.ok) {
            throw new Error(res.statusText);
        }
        return res.json();
    });

/**
 * Auth-aware fetcher that returns null on 401 instead of throwing,
 * allowing consumers to distinguish "not logged in" from an actual error.
 */
export async function authFetcher<T>(url: string): Promise<T | null> {
    const res = await fetch(url);

    if (res.status === 401) {
        return null;
    }

    if (!res.ok) {
        throw new Error(res.statusText);
    }

    return res.json();
}

/**
 * SWR mutation fetcher for POST requests with a JSON body.
 * Parses the response body as an ApiError on failure and surfaces its message.
 */
export async function postJsonFetcher<TArg, TResult>(url: string, { arg }: { arg: TArg }): Promise<TResult> {
    const res = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(arg),
    });

    if (!res.ok) {
        const body: ApiError = await res.json().catch(() => ({ error: res.statusText }));
        throw new Error(body.error ?? res.statusText);
    }

    return res.json();
}

/**
 * SWR mutation fetcher for POST requests that return no body (e.g. logout).
 */
export async function postFetcher(url: string): Promise<void> {
    const res = await fetch(url, { method: "POST" });

    if (!res.ok) {
        throw new Error(res.statusText);
    }
}
