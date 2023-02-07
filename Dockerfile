FROM rust:1.67.0-slim-bullseye as builder

WORKDIR /app/relay

COPY . .

RUN  cargo install --path .


FROM debian:bullseye-slim

ARG USERNAME=mirrorx
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN groupadd --gid $USER_GID $USERNAME && \
    useradd --uid $USER_UID --gid $USER_GID -m $USERNAME && \
    mkdir -p /app/relay && \
    chown $USER_UID:$USER_GID /app/relay

WORKDIR /app/relay

USER $USERNAME

COPY --from=builder /usr/local/cargo/bin/relay .
COPY .env .

CMD ["./relay"]
