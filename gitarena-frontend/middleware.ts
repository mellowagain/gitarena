import { NextRequest, NextResponse } from "next/server";

const PROTECTED_PATHS = ["/new", "/import", "/settings", "/notifications", "/admin"];

export function middleware(request: NextRequest) {
    const { pathname } = request.nextUrl;

    const lowercased = pathname.toLowerCase();
    if (pathname !== lowercased) {
        const url = request.nextUrl.clone();
        url.pathname = lowercased;
        return NextResponse.redirect(url, 308);
    }

    const isProtected = PROTECTED_PATHS.some((p) => pathname === p || pathname.startsWith(p + "/"));
    if (isProtected && !request.cookies.has("gitarena-auth")) {
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
    ],
};
