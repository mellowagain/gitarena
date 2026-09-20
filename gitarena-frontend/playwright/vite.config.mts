import path from "path";
import { fileURLToPath } from "url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The app itself runs on Next.js; this server exists only to serve the component gallery.
const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export default defineConfig({
    root: projectRoot,
    plugins: [react(), tailwindcss()],
    resolve: {
        alias: {
            "@": projectRoot,
        },
    },
    server: {
        port: 3100,
    },
});
