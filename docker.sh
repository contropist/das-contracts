#!/bin/bash

# Docker image name
DOCKER_IMAGE="jjy0/ckb-capsule-recipe-rust:2020-9-28"
# Docker container name
DOCKER_CONTAINER="capsule-dev"
# Name of capsule cache volume
CACHE_VOLUME="capsule-cache"

function build() {
    local contract=$1
    local is_release=$2

    if [ ! -d contracts/$contract ]; then
        echo "Contract ${contract} is not exists, please check for spelling errors."
        exit 1
    fi

    if [[ is_release == "--release" ]]; then
        # Build release version
        docker exec -it -w /code/contracts/$contract $DOCKER_CONTAINER bash -c \
            "RUSTFLAGS=\"-Z pre-link-arg=-zseparate-code -Z pre-link-arg=-zseparate-loadable-segments\" cargo build --release --target riscv64imac-unknown-none-elf && ckb-binary-patcher -i /code/target/riscv64imac-unknown-none-elf/release/${contract} -o /code/target/riscv64imac-unknown-none-elf/release/${contract}"
        docker exec -it -w /code $DOCKER_CONTAINER bash -c \
            "cp /code/target/riscv64imac-unknown-none-elf/release/${contract} /code/build/release/"
    else
        # Build debug version
        docker exec -it -w /code/contracts/$contract $DOCKER_CONTAINER bash -c \
            "cargo build --target riscv64imac-unknown-none-elf && ckb-binary-patcher -i /code/target/riscv64imac-unknown-none-elf/debug/${contract} -o /code/target/riscv64imac-unknown-none-elf/debug/${contract}"
        docker exec -it -w /code $DOCKER_CONTAINER bash -c \
            "cp /code/target/riscv64imac-unknown-none-elf/debug/${contract} /code/build/debug/"
    fi
}

function build_all() {
    dirs=$(ls -a contracts)
    for contract in $dirs; do
        if [[ $contract != "." && $contract != ".." && -d contracts/$contract ]]; then
            build $contract $1
        fi
    done
}

case $1 in
start)
    dir="$(dirname $PWD)"
    docker run -it --rm \
        --name $DOCKER_CONTAINER \
        -v ${dir}/das-contracts:/code \
        -v ${dir}/das-types:/das-types \
        -v $CACHE_VOLUME:/root/.cargo \
        -e RUSTFLAGS="-Z pre-link-arg=-zseparate-code -Z pre-link-arg=-zseparate-loadable-segments" \
        -e CAPSULE_TEST_ENV=debug \
        $DOCKER_IMAGE bin/bash
    ;;
build)
    if [[ $2 == "--release" || $3 == "--release" ]]; then
        if [[ ! -d ./build/release ]]; then
            mkdir -p ./build/release
        fi
    else
        if [[ ! -d ./build/debug ]]; then
            mkdir -p ./build/debug
        fi
    fi

    if [[ -z $2 || $2 == "--release" ]];then
        build_all $2
    else
        build $2 $3
    fi
    ;;
test)
    docker exec -it -w /code $DOCKER_CONTAINER bash -c "cargo test -p tests -- --nocapture"
    ;;
*)
    echo "Unsupported capsule command."
    exit 0
    ;;
esac

echo "Done ✔"
