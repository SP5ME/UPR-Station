FROM rust:1.88-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY . .
RUN cargo build --release --locked --bin ultimatepr-station

FROM alpine:3.22
RUN addgroup -S station && adduser -S -G station station \
    && mkdir -p /var/lib/ultimatepr \
    && chown -R station:station /var/lib/ultimatepr
COPY --from=build /src/target/release/ultimatepr-station /usr/local/bin/ultimatepr-station
ENV RUST_BACKTRACE=0
USER station
WORKDIR /var/lib/ultimatepr
EXPOSE 8080
VOLUME ["/var/lib/ultimatepr"]
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD wget -q -O - http://127.0.0.1:8080/healthz || exit 1
ENTRYPOINT ["/usr/local/bin/ultimatepr-station"]
CMD ["--bind", "0.0.0.0:8080", "--config", "/var/lib/ultimatepr/station.conf", "--web-config", "/var/lib/ultimatepr/web.conf"]
