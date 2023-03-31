FROM alpine

RUN apk add --no-cache curl build-base pkgconfig openssl-dev

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build

COPY . .

RUN rustup target add x86_64-unknown-linux-musl

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine

RUN apk add --no-cache ca-certificates libgcc

COPY --from=0 /build/target/x86_64-unknown-linux-musl/release/gptcli /usr/local/bin/gptcli

ENTRYPOINT ["/usr/local/bin/gptcli"]
