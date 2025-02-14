#!/bin/sh -x

install_package (){
    sudo apt update
    sudo apt-get install -y binutils
    sudo apt-get install -y llvm
    sudo apt-get install -y gcc-aarch64-linux-gnu
    sudo apt-get install -y musl
    sudo apt-get install -y musl-tools
}
set_gcc_prefix(){
    case "${1}" in
        aarch64-*)
            GCC_PREFIX="aarch64-linux-gnu-"
        ;;
        *)
            GCC_PREFIX=""
        ;;
    esac
    
    
}

compile(){
    set_gcc_prefix $1
    
    rustup target add $1
    eval cargo +stable build --release --target $1
}

main(){
    install_package
    # cargo clean
    # compile x86_64-apple-darwin
    # compile aarch64-apple-darwin
    compile x86_64-unknown-linux-gnu
    compile x86_64-unknown-linux-musl
    # compile aarch64-unknown-linux-gnu
    # compile aarch64-unknown-linux-musl
    # compile i686-unknown-linux-gnu
    # compile i686-unknown-linux-musl
    # compile x86_64-pc-windows-msvc
}

main