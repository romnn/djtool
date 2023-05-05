ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC apt-get -y install tzdata

RUN apt-get install -y \
    llvm-dev libclang-dev clang \
    build-essential \
    curl \
    wget \
    webkit2gtk-4.0 \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libsoup2.4 \
    libasound2-dev \
    yasm \
    protobuf-compiler

