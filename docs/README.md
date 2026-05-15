# GitArena Docs

Documentation for [GitArena](https://github.com/mellowagain/gitarena), a self-hosted software development platform with git hosting, user accounts, and instance administration.

Built with [Mintlify](https://mintlify.com).

## Local preview

Install the Mintlify CLI:

```bash
npm i -g mint
```

Run the preview server from the `docs/` directory:

```bash
mint dev
```

Open `http://localhost:3000` to see the docs.

## Check links

```bash
mint broken-links
```

## Structure

```
docs/
├── docs.json           # Site config (navigation, theme, colors)
├── index.mdx           # Introduction
├── quickstart.mdx      # Docker deployment
├── development.mdx     # Local dev setup
├── architecture/       # Backend, frontend, background jobs, SSH
├── features/           # Repositories, auth, issues, admin, explore
├── self-hosting/       # Docker, configuration, reverse proxy, SSO, email, CAPTCHA
├── api-reference/      # API overview (OpenAPI auto-generation)
└── contributing/       # Dev setup, backend/frontend patterns, background jobs
```

## Publishing

Connect the repository to [Mintlify](https://mintlify.com) via the dashboard. Changes pushed to the default branch deploy automatically.
