FROM ubuntu:20.04

# RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

RUN apt-get update
RUN apt-get install -y build-essential curl

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN export PATH="$PATH:$HOME/.cargo/env"

WORKDIR /build
ADD . /build

RUN cargo build
