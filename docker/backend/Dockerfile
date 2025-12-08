FROM rust:1.87.0-alpine AS base

WORKDIR /usr/src/app

RUN set -eux; \
    apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static; \
    rm -rf $CARGO_HOME/registry

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/backend crates/backend

# Create dummy ssg crate for workspace build
RUN mkdir -p crates/ssg/src && \
    echo '[package]\nname = "blog-ssg"\nversion = "0.1.0"\nedition = "2021"\n\n[[bin]]\nname = "blog"\npath = "src/main.rs"' > crates/ssg/Cargo.toml && \
    echo 'fn main() {}' > crates/ssg/src/main.rs

# Build dependencies (cache layer)
RUN mkdir -p crates/backend/src && \
    echo 'fn main() {println!("placeholder");}' > crates/backend/src/main.rs && \
    cargo build -p blog-backend --release && \
    rm target/release/blog-api* && \
    rm target/release/deps/blog_backend* && \
    rm -rf crates/backend/src

FROM base AS builder

COPY crates/backend/src crates/backend/src
RUN cargo build -p blog-backend --release

FROM alpine:3.20.2

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/app/target/release/blog-api .

EXPOSE ${PORT}

CMD ["./blog-api"]
