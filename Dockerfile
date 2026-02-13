FROM rust:1.91 AS builder

RUN cargo install wasm-pack
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/

RUN wasm-pack build --target web --release -- --features wasm

FROM nginx:alpine

# Preserve the same directory structure as local dev:
#   /usr/share/nginx/html/web/index.html  → serves at /web/
#   /usr/share/nginx/html/pkg/            → serves at /pkg/
# So the import '../pkg/pdf_parser.js' works in both environments.
COPY web/ /usr/share/nginx/html/web/
COPY --from=builder /app/pkg/ /usr/share/nginx/html/pkg/
COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080
