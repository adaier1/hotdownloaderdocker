# um_crypto

加解密算法实现 (Rust)。

## 项目结构

```text
lib_um_crypto_rust
├── um_cli          用于调试的 CLI 方案。
├── um_crypto/*     子目录存放具体算法的代码。
├── um_wasm         将 um_crypto 包装为能从 JavaScript 层调用的代码。
└── um_wasm_loader  打包 `um_wasm` 到 npm 包的代码。
```

## 构建说明

利用 Docker 构建（推荐）：

```
bash build.sh
```

### 单独构建 um_wasm

首先安装 `wasm-pack`：

```sh
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

然后构建：

```sh
cd um_wasm
wasm-pack build --release --target web
```

注意此处构建的产出不会被用于 NPM 包。

### 构建 NPM 包

安装 `wasm-pack` 以及 `pnpm`，然后构建：

```sh
cd um_wasm_loader
pnpm build
```

## 作 NPM 包使用

参考 [`um_wasm_loader/Readme.MD`](um_wasm_loader/Readme.MD) 的说明。

## 授权协议

Apache License 2.0 + MIT 双协议
