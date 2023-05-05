# FROM rust:latest AS build
FROM ghcr.io/cross-rs/x86_64-unknown-linux-gnu AS build
# FROM ubuntu:20.04

RUN echo 'source $HOME/.cargo/env' >> "$HOME/.bashrc"

RUN apt-get update

# install rust
RUN apt-get install -y build-essential curl

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="$PATH:$HOME/.cargo/env"

# install tauri ui dependencies
RUN apt-get install -y \
	build-essential \
	curl \
	wget \
	webkit2gtk-4.0 \
	libssl-dev \
	libgtk-3-dev \
	librsvg2-dev

# libayatana-appindicator3-dev \

# install other dependencies (Todo check for what)
RUN apt-get install -y \
	libsoup2.4

# install audio dependencies for cpal playback
RUN apt-get install -y \
	libasound2-dev

# install ffmpeg dependencies
RUN apt-get install -y \
	build-essential \
	llvm-dev libclang-dev clang \
	yasm

# install prost dependencies
RUN apt-get install -y \
	protobuf-compiler

WORKDIR /build
ADD . /build

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo build

FROM gcr.io/distroless/cc
COPY --from=build /build/target/debug/djtool /
CMD ["./djtool"]
