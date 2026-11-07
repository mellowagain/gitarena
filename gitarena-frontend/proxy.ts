import { NextRequest, NextResponse } from "next/server";

const PROTECTED_PATHS = ["/new", "/import", "/settings", "/notifications", "/admin"];

const BACKEND_URL = process.env.BACKEND_URL ?? process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

export function proxy(request: NextRequest) {
    const { pathname, search } = request.nextUrl;

    if (pathname === "/api" || pathname.startsWith("/api/")) {
        return NextResponse.rewrite(new URL(BACKEND_URL.replace(/\/$/, "") + pathname + search));
    }

    // File paths in tree and commits routes are case-sensitive — skip lowercasing for those
    const CASE_SENSITIVE_PREFIXES = [/^\/[^/]+\/[^/]+\/tree\//, /^\/[^/]+\/[^/]+\/commits\//];
    if (!pathname.startsWith("/api/") && !CASE_SENSITIVE_PREFIXES.some((re) => re.test(pathname))) {
        const lowercased = pathname.toLowerCase();
        if (pathname !== lowercased) {
            const url = request.nextUrl.clone();
            url.pathname = lowercased;
            // Redirecting RSC prefetches makes the router refetch the original href forever, so rewrite them instead.
            // Next strips the `rsc` header before the proxy runs, so check Sec-Fetch-Dest instead
            if (request.headers.get("sec-fetch-dest") === "empty") {
                return NextResponse.rewrite(url);
            }
            return NextResponse.redirect(url, 308);
        }
    }

    const isProtected = PROTECTED_PATHS.some((p) => pathname === p || pathname.startsWith(p + "/"));
    if (isProtected && !request.cookies.has("gitarena-session")) {
        const url = request.nextUrl.clone();
        url.pathname = "/login";
        url.searchParams.set("redirect", pathname);
        return NextResponse.redirect(url);
    }

    return NextResponse.next();
}

export const config = {
    matcher: [
        /*
         * Match all paths except:
         *  - _next (Next.js internals: static files, image optimization, etc.)
         *  - favicon files
         *  - public folder assets (images, icons, etc.)
         */
        "/((?!_next|favicon|.*\\.(?:ico|png|svg|jpg|jpeg|gif|webp|woff|woff2|ttf|otf|css|js|map)$).*)",
        // The pattern above skips static file extensions, which api paths can end in too
        "/api/:path*",
    ],
};
