FROM alpine
RUN apk add --no-cache ca-certificates

FROM scratch
COPY --from=0 /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY target/x86_64-unknown-linux-musl/release/gptcli /usr/local/bin/gptcli
ENTRYPOINT ["/usr/local/bin/gptcli"]