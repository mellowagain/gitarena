/** @type {import('next').NextConfig} */
const nextConfig = {
    // only the docker image needs this, vercel has its own build output
    output: process.env.NEXT_STANDALONE === "true" ? "standalone" : undefined,
    typescript: {
        ignoreBuildErrors: true,
    },
    images: {
        unoptimized: true,
    },
};

export default nextConfig;
