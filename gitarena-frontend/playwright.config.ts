import { defineConfig, devices } from "@playwright/test";

const galleryUrl = "http://localhost:3100/playwright/gallery/index.html";

export default defineConfig({
    projects: [
        {
            name: "components",
            testDir: "./tests/components",
            use: {
                ...devices["Desktop Chrome"],
                baseURL: galleryUrl,
                serviceWorkers: "block",
                reuseContext: true,
            },
        },
    ],
    webServer: {
        command: "pnpm exec vite --config playwright/vite.config.mts",
        url: galleryUrl,
        reuseExistingServer: !process.env.CI,
    },
});
