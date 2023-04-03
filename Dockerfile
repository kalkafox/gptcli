FROM alpine

RUN apk add --no-cache libgcc openssl

COPY target/release/gptcli /usr/local/bin/gptcli
ENTRYPOINT ["/usr/local/bin/gptcli"]