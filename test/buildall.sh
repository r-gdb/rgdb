#!/bin/sh -x

install_package (){
    
    case "$1" in
        aarch64-*)
            sudo apt update
            sudo apt install -y binutils
            sudo apt-get install -y llvm
            sudp apt install -y gcc-aarch64-linux-gnu
            ;;
    esac
    case "$1" in
        *-musl)
            sudo apt update
            sudo apt-get install -y musl
            sudo apt-get install -y musl-tools
            sudo apt-get install -y llvm
            ;;
    esac
}
set_gcc_prefix(){
    case "${1}" in
        aarch64-*)
            export GCC_PREFIX="aarch64-linux-gnu-"
        ;;
    esac
    

}

compile(){
    set_gcc_prefix $1
    install_package $1
    rustup target add $1
    eval cargo +stable build --release --target $1
}

# cargo clean
# compile x86_64-apple-darwin
# compile aarch64-apple-darwin
compile x86_64-unknown-linux-gnu
compile x86_64-unknown-linux-musl
compile aarch64-unknown-linux-gnu
compile ubuntu-latest-aarch64-musl
compile i686-unknown-linux-gnu
compile i686-unknown-linux-musl
# compile x86_64-pc-windows-msvc

