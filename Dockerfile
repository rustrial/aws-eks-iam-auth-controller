ARG ALPINE_VERSION=3.14.0

FROM alpine:$ALPINE_VERSION as builder

RUN apk --no-cache add ca-certificates libgcc gcc pkgconfig openssl-dev build-base curl

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH=$PATH:/root/.cargo/bin

WORKDIR /workdir

COPY . /workdir/

# Needed to fix double linking of openssl https://users.rust-lang.org/t/sigsegv-with-program-linked-against-openssl-in-an-alpine-container/52172/3
# https://rust-lang.github.io/rfcs/1721-crt-static.html
ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN cargo build --release

FROM alpine:$ALPINE_VERSION

# Needed for RUSTFLAGS="-C target-feature=-crt-static" as above
RUN apk --no-cache add libgcc

# Cross compile arm64/aarch64 binaries created by docker buildx are linked to /lib/ld-linux-aarch64.so.1 
# which is called /lib/ld-musl-aarch64.so.1 on alpine.
RUN ( [[ $(uname -m) == "aarch64" ]] && ln -s /lib/ld-musl-aarch64.so.1 /lib/ld-linux-aarch64.so.1 ) || true

COPY --from=builder /workdir/target/release/rustrial-aws-eks-iam-auth-controller /usr/local/bin/rustrial-aws-eks-iam-auth-controller

ENTRYPOINT [ "/usr/local/bin/rustrial-aws-eks-iam-auth-controller" ]

USER 1000