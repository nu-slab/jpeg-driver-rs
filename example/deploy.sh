#!/bin/bash
set -ue

cargo_target=aarch64-unknown-linux-gnu
app_name=jpeg_driver-rs
ssh_remote=remote_castella
deploy_dir=/home/ubuntu/work/jpeg_encoder

if [[ $(uname -r) =~ ^.+xilinx-zynqmp$ ]]; then
    echo On board
    cargo b -r
    mkdir -p $deploy_dir
    cp target/release/$app_name $deploy_dir
else
    if [[ $(rustc -vV | sed -n 's|host: ||p') == $cargo_target ]]; then
        echo Same arch
        cargo b -r
        rsync -auvP target/release/$app_name $ssh_remote:$deploy_dir/$app_name
    else
        echo Cross compile
        if !(type "cross" > /dev/null 2>&1); then
            echo Install cross
            cargo install cross --git https://github.com/cross-rs/cross
        fi
        cross build --target $cargo_target -r  
        rsync -auvP target/$cargo_target/release/$app_name $ssh_remote:$deploy_dir/$app_name
        # cross build --target $cargo_target
        # rsync -auvP target/$cargo_target/debug/$app_name $ssh_remote:$deploy_dir/$app_name
    fi

fi
