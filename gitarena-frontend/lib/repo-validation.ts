/**
 * Validates a URL string using the URL constructor (same behaviour as Rust's url::Url::parse).
 * Returns true when the URL is structurally valid.
 */
export function isValidUrl(url: string): boolean {
    try {
        new URL(url);
        return true;
    } catch {
        return false;
    }
}

/**
 * Attempts to extract a repository name from a clone URL.
 * e.g. "https://github.com/user/my-repo.git" → "my-repo"
 */
export function extractRepoNameFromUrl(url: string): string {
    const parts = url.split("/");
    return (parts[parts.length - 1] ?? "").replace(/\.git$/, "");
}
