/** @type {import('next').NextConfig} */
const nextConfig = {
    typescript: {
        ignoreBuildErrors: true,
    },
    images: {
        unoptimized: true,
    },
    async rewrites() {
        const backendUrl = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";
        return [
            {
                source: "/api/:path*",
                destination: `${backendUrl}/api/:path*`,
            },
            {
                source: "/:username/:repo.git/:path*",
                destination: `${backendUrl}/:username/:repo.git/:path*`,
            },
        ];
    },
};

export default nextConfig;
