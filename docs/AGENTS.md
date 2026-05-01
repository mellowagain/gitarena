# Documentation project instructions

## About this project

- This is the GitArena documentation site, built on [Mintlify](https://mintlify.com)
- Pages are MDX files with YAML frontmatter
- Configuration lives in `docs.json`
- Run `mint dev` to preview locally
- Run `mint broken-links` to check internal links

## What is GitArena

GitArena is a self-hosted software development platform — a lightweight alternative to GitLab and Gitea. It includes:

- Git repository hosting over HTTP (smart protocol) and SSH
- Issue tracking and merge requests
- User accounts with passkeys, SSO, and session management
- An admin panel for instance management

## Terminology

Use these terms consistently:

| Preferred      | Avoid                          |
|----------------|--------------------------------|
| merge request  | pull request                   |
| repository     | repo (in prose; fine in code)  |
| instance       | server, installation           |
| admin panel    | admin dashboard, control panel |
| passkey        | WebAuthn credential            |
| settings table | config table, configuration    |
| workhorse      | background worker              |
| smart HTTP     | git HTTP protocol              |

## Site structure

| Tab | What it covers |
|-----|---------------|
| Guides | Getting started, architecture, features, self-hosting, contributing |
| API reference | REST API (auto-generated from OpenAPI + intro page) |

## Style preferences

- Active voice and second person ("you")
- Sentence case for headings
- Concise sentences — one idea per sentence
- Bold for UI elements: Click **Settings**
- Code formatting for file names, commands, paths, variables, and settings keys
- No marketing language ("powerful", "seamless", "robust")
- No filler phrases ("it's important to note", "in order to")

## Content boundaries

Document:
- Features that exist and work
- Configuration that is available in the settings table
- Architecture that is implemented

Note clearly with a `<Note>` callout:
- Features that are under active development or not yet complete
- SSH git access (key lookup works; git over SSH is not yet fully implemented)

Do not document:
- Planned features not yet started
- Internal admin-only tooling not exposed in the UI or API
