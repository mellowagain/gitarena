ARG NODE_VERSION=24.14.1-slim

FROM node:${NODE_VERSION} AS base

ENV NEXT_TELEMETRY_DISABLED=1
RUN corepack enable && corepack prepare pnpm@10.33.0 --activate
WORKDIR /app

FROM base AS dependencies

COPY package.json pnpm-lock.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store pnpm install --frozen-lockfile \
    && pnpm dlx generate-license-file@4.2.5 --input package.json --output THIRD_PARTY_LICENSES.txt

FROM base AS builder

ENV NEXT_STANDALONE=true
ENV NODE_ENV=production

COPY --from=dependencies /app/node_modules ./node_modules
COPY . .

RUN --mount=type=cache,target=/app/.next/cache pnpm build

FROM node:${NODE_VERSION} AS runner

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3000
ENV HOSTNAME=0.0.0.0
WORKDIR /app

COPY --from=builder --chown=node:node /app/public ./public
COPY --from=builder --chown=node:node /app/.next/standalone ./
COPY --from=builder --chown=node:node /app/.next/static ./.next/static
COPY --from=dependencies --chown=node:node /app/THIRD_PARTY_LICENSES.txt ./THIRD_PARTY_LICENSES.txt
COPY --from=license --chown=node:node LICENSE.txt ./LICENSE.txt

USER node
EXPOSE 3000
CMD ["node", "server.js"]
