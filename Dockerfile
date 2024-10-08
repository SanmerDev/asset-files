FROM --platform=$BUILDPLATFORM rust:alpine AS builder
COPY . /rust/asset-files
WORKDIR /rust
RUN set -ex \
    && apk add libc-dev \
    && cargo install --path asset-files

FROM alpine AS dist
COPY --from=builder /usr/local/cargo/bin/asset-files /usr/local/bin/asset-files
EXPOSE 8080
ENTRYPOINT ["asset-files"]