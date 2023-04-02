FROM busybox
COPY target/release/gptcli /usr/local/bin/gptcli
ENTRYPOINT ["/usr/local/bin/gptcli"]