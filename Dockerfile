FROM ubuntu:22.04

SHELL ["/bin/bash", "--login", "-c"]

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    libssl-dev \
    zlib1g \
    zlib1g-dev \
    libuv1-dev \
    python3 \
    help2man \
    perl \
    flex \
    bison \
    ccache \
    autoconf \
    libgoogle-perftools-dev \
    numactl \
    perl-doc \
    libfl2 \
    libfl-dev \
    default-jre \
    default-jdk \
    python3-pip \
    && apt-get clean

# Install Rust toolchain
# ENV PATH="$PATH:$HOME/.cargo/bin"
RUN cd $HOME \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && echo 'export PATH="${PATH}:$HOME/.cargo/bin"' >> ~/.bashrc \
    && export PATH="${PATH}:$HOME/.cargo/bin" \
    && rustup default nightly-2023-11-16 \
    && rustup target add aarch64-unknown-none

# Install verilator
RUN cd $HOME \
    && git clone https://github.com/verilator/verilator \
    && cd $HOME/verilator \
    && git pull \
    && git checkout v5.014 \
    && ccache -M 50G \
    && autoconf \
    && ./configure \
    && make -j `nproc` \
    && make install \
    && verilator --version

# Install SBT for Scala
# ENV PATH="$PATH:$HOME/.local/share/coursier/bin"
ARG CS_URL="https://github.com/coursier/coursier/releases/latest/download/cs-x86_64-pc-linux.gz"
RUN cd $HOME \
    && curl -fL $CS_URL | gzip -d > cs \
    && chmod +x cs \
    && ./cs setup -y \
    && echo 'export PATH="${PATH}:$HOME/.local/share/coursier/bin"' >> ~/.bashrc \
    && export PATH="${PATH}:$HOME/.local/share/coursier/bin"

# Install Python dependencies
RUN pip install dataclasses-json hjson numpy protobuf scipy gitpython matplotlib

# Download Fusion Blossom and put it aside the micro-blossom folder
ARG FUSION_BLOSSOM_COMMIT="c90d75362e7994f90b1199af3ec648ae6ebc034b"
RUN cd $HOME \
    && git clone https://github.com/yuewuo/fusion-blossom.git \
    && cd $HOME/fusion-blossom \
    && git checkout $FUSION_BLOSSOM_COMMIT \
    && cargo build --release

# Export several useful shortcuts
RUN echo 'export MB_CIRCUIT_LEVEL_FINAL="$HOME/micro-blossom/benchmark/hardware/frequency_optimization/circuit_level_final"' >> ~/.bashrc \
    && echo 'export MB_VIVADO_PROJECTS="$MB_CIRCUIT_LEVEL_FINAL/tmp-project"' >> ~/.bashrc 
