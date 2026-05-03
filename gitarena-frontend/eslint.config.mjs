import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";
import prettier from "eslint-config-prettier/flat";

const eslintConfig = defineConfig([
    ...nextVitals,
    ...nextTs,
    prettier,
    globalIgnores([
        // Default ignores of eslint-config-next:
        ".next/**",
        "out/**",
        "build/**",
        "next-env.d.ts",
    ]),
    {
        settings: {
            react: {
                // keep in sync with package.json
                version: "19",
            },
        },
    },
    {
        rules: {
            curly: ["error", "all"],
            // /docs is a Vercel rewrite to an external host (Mintlify), not a Next.js page.
            // <a> is intentional here to force a full page load through Vercel's rewrite layer.
            "@next/next/no-html-link-for-pages": "off",
        },
    },
]);

export default eslintConfig;
