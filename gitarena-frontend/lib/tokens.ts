export type TokenType = "personal" | "organization" | "ci" | "deploy" | "runner" | "instance";

export type TokenScope = "all" | "public" | "selected";

export interface TokenResponse {
    id: string;
    name: string;
    ownerUser: string | null;
    ownerOrg: string | null;
    ownerRepo: string | null;
    creator: string | null;
    tokenType: TokenType;
    scope: TokenScope;
    permissions: string[];
    lastUsedAt: string | null;
    expiresAt: string | null;
    revokedAt: string | null;
    scopeOrgs: string[];
    scopeRepos: string[];
}

export interface CreateTokenResponse {
    token: TokenResponse;
    secret: string;
}

export interface CreateTokenRequest {
    name: string;
    tokenType: TokenType;
    scope: TokenScope;
    permissions: string[];
    /** Unix timestamp in seconds, or null for a token that never expires */
    expiresAt: number | null;
    ownerOrg?: string;
    ownerRepo?: string;
    scopeOrgs: string[];
    scopeRepos: string[];
}

export interface UpdateTokenRequest {
    name: string;
    permissions: string[];
    scope: TokenScope;
    scopeOrgs: string[];
    scopeRepos: string[];
}

/** Who a token belongs to, which decides the API owner filter and the types that can be created. */
export type TokenOwner =
    | { kind: "user"; username: string }
    | { kind: "org"; id: string; name: string }
    | { kind: "repo"; id: string }
    | { kind: "instance" };

export const tokenTypeLabels: Record<TokenType, string> = {
    personal: "Personal",
    organization: "Organization",
    ci: "CI",
    deploy: "Deploy",
    runner: "Runner",
    instance: "Instance",
};

export const tokenScopeLabels: Record<TokenScope, string> = {
    all: "All resources",
    public: "Public resources only",
    selected: "Selected resources",
};

export function tokenListKey(owner: TokenOwner, includeRevoked: boolean): string {
    const params = new URLSearchParams();

    if (owner.kind === "org") {
        params.set("org", owner.id);
    } else if (owner.kind === "repo") {
        params.set("repo", owner.id);
    } else if (owner.kind === "instance") {
        params.set("instance", "true");
    }

    if (includeRevoked) {
        params.set("includeRevoked", "true");
    }

    const query = params.toString();
    return query ? `/api/tokens?${query}` : "/api/tokens";
}

/** Token types that can be created for an owner. */
export function tokenTypesFor(owner: TokenOwner): TokenType[] {
    switch (owner.kind) {
        case "user":
            return ["personal"];
        case "org":
            return ["organization"];
        case "repo":
            return ["deploy"];
        case "instance":
            return ["instance", "runner"];
    }
}

/** Mirrors the `tokens_scope_valid` constraint in the backend. */
export function scopesFor(tokenType: TokenType): TokenScope[] {
    switch (tokenType) {
        case "personal":
            return ["all", "public", "selected"];
        case "organization":
            return ["all", "selected"];
        default:
            return ["all"];
    }
}
