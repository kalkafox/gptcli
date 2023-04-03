FROM alpine

RUN apk add --no-cache libgcc openssl

COPY target/x86_64-unknown-linux-musl/release/gptcli /usr/local/bin/gptcli
ENTRYPOINT ["/usr/local/bin/gptcli"]