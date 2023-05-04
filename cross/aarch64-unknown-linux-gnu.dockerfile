ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

RUN dpkg --add-architecture arm64 && \
    apt-get update
RUN apt-get install -y \
    llvm-dev libclang-dev clang:arm64 \
    build-essential:arm64 \
    curl:arm64 \
    wget:arm64 \
    webkit2gtk-4.0:arm64 \
    libssl-dev:arm64 \
    libgtk-3-dev:arm64 \
    libayatana-appindicator3-dev:arm64 \
    librsvg2-dev:arm64 \
    libsoup2.4:arm64 \
    libasound2-dev:arm64 \
    yasm:arm64 \
    protobuf-compiler:arm64

