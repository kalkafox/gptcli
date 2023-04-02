FROM busybox
COPY /build/target/release/gptcli /usr/local/bin/gptcli
ENTRYPOINT ["/usr/local/bin/gptcli"]