#!/bin/bash -ex

docker volume create umc_rust_cache
docker volume create umc_rust_pnpm_cache
docker volume create umc_rust_target_cache
docker volume create umc_pnpm_loader_cache
docker buildx build --build-arg uid="$(id -u)" --build-arg gid="$(id -g)" -t umc_rust .

run_container() {
    docker run -d \
        -v umc_rust_cache:/h/.cargo \
        -v "${PWD}:/a" \
        -v umc_rust_pnpm_cache:/a/.pnpm-store \
        -v umc_rust_target_cache:/a/target \
        -v umc_pnpm_loader_cache:/a/um_wasm_loader/node_modules \
        umc_rust
}

umc_rust_id="$(run_container)"

docker exec -i -u root "${umc_rust_id}" chown "$(id -u):$(id -g)" \
    /h/.cargo \
    /a/.pnpm-store \
    /a/target \
    /a/um_wasm_loader/node_modules

docker exec -i "${umc_rust_id}" bash <<'EOF'
    export npm_config_use_node_version=22.9.0
    cd um_wasm_loader
    pnpm --package-import-method=copy i
    pnpm build
    pnpm pack
EOF
docker stop "${umc_rust_id}"
docker rm -f "${umc_rust_id}"
