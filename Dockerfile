FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile
COPY frontend ./
RUN pnpm build

FROM rust:1-bookworm AS backend-builder
WORKDIR /app
ARG SERVICECOMPASS_VERSION
COPY Cargo.toml Cargo.lock ./
COPY backend backend
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  SERVICECOMPASS_VERSION="${SERVICECOMPASS_VERSION}" cargo build --release -p service-compass-backend

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl \
  && rm -rf /var/lib/apt/lists/*
COPY --from=backend-builder /app/target/release/service-compass /usr/local/bin/service-compass
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
ENV DATABASE_URL=sqlite:/data/service-compass.db \
    SERVICECOMPASS_BIND=0.0.0.0:3000 \
    SERVICECOMPASS_PRODUCTION=true
VOLUME ["/data"]
EXPOSE 3000
CMD ["service-compass"]
