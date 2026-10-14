FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev python3
WORKDIR /src
COPY . .
RUN cargo build --release --locked --bin ultimatepr-station
RUN python3 scripts/desktop-licenses.py --manifest Cargo.toml --output target/server-licenses \
    && test ! -e target/server-licenses/MISSING-LICENSE-TEXTS.txt

FROM alpine:3.22
RUN apk add --no-cache ca-certificates \
    && addgroup -S station && adduser -S -G station station \
    && mkdir -p /var/lib/ultimatepr \
    && chown -R station:station /var/lib/ultimatepr
COPY --from=build /src/target/release/ultimatepr-station /usr/local/bin/ultimatepr-station
COPY --from=build /src/target/server-licenses/ /usr/share/licenses/upr-station/crates/
COPY LICENSE /usr/share/licenses/upr-station/LICENSE
COPY licenses/MDI-LICENSE.txt licenses/Apache-2.0.txt /usr/share/licenses/upr-station/assets/
ENV RUST_BACKTRACE=0
USER station
WORKDIR /var/lib/ultimatepr
EXPOSE 8080
VOLUME ["/var/lib/ultimatepr"]
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD wget -q -O - http://127.0.0.1:8080/healthz || exit 1
ENTRYPOINT ["/usr/local/bin/ultimatepr-station"]
CMD ["--bind", "0.0.0.0:8080", "--config", "/var/lib/ultimatepr/station.conf", "--web-config", "/var/lib/ultimatepr/web.conf"]
