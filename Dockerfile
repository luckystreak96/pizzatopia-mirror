FROM rust:1.39.0

# Install Cross-compiler toolchain
RUN apt-get update -yq
RUN apt-get install -y clang cmake cpio make libssl-dev lzma-dev libxml2-dev sed
RUN rustup target add x86_64-apple-darwin

RUN mkdir -p /mac_build
RUN cd /mac_build && git clone --depth 1 https://github.com/tpoechtrager/osxcross.git
RUN cd /mac_build/osxcross/tarballs && wget https://s3.dockerproject.org/darwin/v2/MacOSX10.11.sdk.tar.xz
RUN cd /mac_build/osxcross && \
    UNATTENDED=yes OSX_VERSION_MIN=10.7 ./build.sh && \
    export PATH="$PATH:/mac_build/osxcross/target/bin" && \
    ln -s /mac_build/osxcross/target/SDK/MacOSX10.11.sdk/System/ /System
