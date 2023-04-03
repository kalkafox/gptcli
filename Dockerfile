FROM scratch
COPY target/x86_64-unknown-linux-musl/release/gptcli /usr/local/bin/gptcli
ENTRYPOINT ["/usr/local/bin/gptcli"]