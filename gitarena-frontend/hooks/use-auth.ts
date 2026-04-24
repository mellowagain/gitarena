import useSWR from "swr";
import useSWRMutation from "swr/mutation";

export interface AuthUser {
    id: number;
    username: string;
    admin: boolean;
}

interface ApiError {
    error: string;
}

async function authFetcher(url: string): Promise<AuthUser | null> {
    const res = await fetch(url);

    if (res.status === 401) {
        return null;
    }

    if (!res.ok) {
        throw new Error(res.statusText);
    }

    return res.json();
}

async function loginFetcher(url: string, { arg }: { arg: { identifier: string; password: string } }): Promise<AuthUser> {
    const res = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(arg),
    });

    if (!res.ok) {
        const body: ApiError = await res.json().catch(() => ({ error: "Login failed" }));
        throw new Error(body.error ?? "Login failed");
    }

    return res.json();
}

async function registerFetcher(_url: string, { arg }: { arg: { username: string; email: string; password: string } }): Promise<AuthUser> {
    const res = await fetch("/api/user", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(arg),
    });

    if (!res.ok) {
        const body: ApiError = await res.json().catch(() => ({ error: "Registration failed" }));
        throw new Error(body.error ?? "Registration failed");
    }

    return res.json();
}

async function logoutFetcher(url: string): Promise<void> {
    const res = await fetch(url, { method: "POST" });

    if (!res.ok) {
        throw new Error(`Logout failed: ${res.statusText}`);
    }
}

export function useAuth() {
    const { data, error, isLoading, mutate } = useSWR<AuthUser | null>("/api/auth/me", authFetcher, {
        shouldRetryOnError: false,
        revalidateOnFocus: true,
    });

    const { trigger: triggerRegister, isMutating: isRegistering, error: registerError } = useSWRMutation("/api/user", registerFetcher, {
        onSuccess: (user) => mutate(user, { revalidate: false }),
    });

    const { trigger: triggerLogin, isMutating: isLoggingIn, error: loginError } = useSWRMutation("/api/auth/login", loginFetcher, {
        onSuccess: (user) => mutate(user, { revalidate: false }),
    });

    const { trigger: triggerLogout, isMutating: isLoggingOut, error: logoutError } = useSWRMutation("/api/auth/logout", logoutFetcher, {
        onSuccess: () => mutate(null, { revalidate: false }),
    });

    return {
        user: data ?? null,
        error: error as Error | undefined,
        isLoading,
        isAuthenticated: data != null,
        register: (username: string, email: string, password: string) => triggerRegister({ username, email, password }),
        isRegistering,
        registerError: registerError as Error | undefined,
        login: (identifier: string, password: string) => triggerLogin({ identifier, password }),
        isLoggingIn,
        loginError: loginError as Error | undefined,
        logout: triggerLogout,
        isLoggingOut,
        logoutError: logoutError as Error | undefined,
        mutate,
    };
}
