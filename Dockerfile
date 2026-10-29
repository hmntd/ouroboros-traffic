FROM rust:1.85-slim-bookworm AS wasm-builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates pkg-config libssl-dev && \
    curl -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN wasm-pack build --target web --release

FROM node:22-alpine AS ui-builder

WORKDIR /app

COPY --from=wasm-builder /app/pkg /app/pkg

WORKDIR /app/ui
COPY ui/package*.json ./
RUN npm ci || npm install

COPY ui/ ./
RUN npm run build

FROM nginx:alpine AS runner

COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=ui-builder /app/ui/dist /usr/share/nginx/html

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
