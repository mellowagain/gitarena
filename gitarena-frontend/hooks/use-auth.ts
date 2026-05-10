import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { authFetcher, postJsonFetcher, postFetcher } from "@/lib/fetchers";

export interface AuthUser {
    id: string;
    username: string;
    admin: boolean;
}

interface LoginArgs {
    identifier: string;
    password: string;
}

interface RegisterArgs {
    username: string;
    email: string;
    password: string;
}

export function useAuth() {
    const { data, error, isLoading, mutate } = useSWR<AuthUser | null>("/api/auth/me", authFetcher, {
        shouldRetryOnError: false,
        revalidateOnFocus: true,
    });

    const {
        trigger: triggerRegister,
        isMutating: isRegistering,
        error: registerError,
    } = useSWRMutation<AuthUser, Error, string, RegisterArgs>("/api/user", postJsonFetcher, {
        onSuccess: (user) => mutate(user, { revalidate: false }),
    });

    const {
        trigger: triggerLogin,
        isMutating: isLoggingIn,
        error: loginError,
    } = useSWRMutation<AuthUser, Error, string, LoginArgs>("/api/auth/login", postJsonFetcher, {
        onSuccess: (user) => mutate(user, { revalidate: false }),
    });

    const {
        trigger: triggerLogout,
        isMutating: isLoggingOut,
        error: logoutError,
    } = useSWRMutation("/api/auth/logout", postFetcher, {
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
