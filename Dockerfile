FROM ubuntu:22.04

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
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN . "$HOME/.cargo/env"
RUN rustup default nightly-2023-11-16
RUN rustup target add aarch64-unknown-none

# Install verilator
WORKDIR $HOME
RUN git clone https://github.com/verilator/verilator
WORKDIR $HOME/verilator
RUN git pull
RUN git checkout v5.014
RUN ccache -M 50G
RUN autoconf
RUN ./configure
RUN make -j `nproc`
RUN make install
RUN verilator --version

# Install SBT for Scala
WORKDIR $HOME
RUN curl -fL https://github.com/coursier/coursier/releases/latest/download/cs-x86_64-pc-linux.gz | gzip -d > cs && chmod +x cs && ./cs setup -y
RUN echo 'export PATH="${PATH}:/root/.local/share/coursier/bin"' >> ~/.bashrc 
RUN source $HOME/.bashrc
WORKDIR $HOME/micro-blossom
RUN sbt version

# Install Python dependencies
RUN pip install -r $HOME/micro-blossom/benchmark/requirements.txt

# Download Fusion Blossom and put it aside the micro-blossom folder
WORKDIR $HOME
RUN git clone https://github.com/yuewuo/fusion-blossom.git
WORKDIR $HOME/fusion-blossom
RUN git checkout c90d75362e7994f90b1199af3ec648ae6ebc034b
RUN cargo build --release

# Build Micro Blossom binary
WORKDIR $HOME/micro-blossom/src/cpu/blossom
RUN cargo build --release
