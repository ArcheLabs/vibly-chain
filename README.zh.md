# vibly-chain

`vibly-chain` 是基于 [Polkadot SDK](https://github.com/paritytech/polkadot-sdk) 构建的 Vibly 协调链，提供两种运行时配置：

| 配置 | 二进制 | 默认 RPC |
|---|---|---|
| **平行链**（`node/` + `runtime/`） | `vibly-chain-node` | 9988 |
| **单节点**（`solo-node/` + `solo-runtime/`） | `vibly-solo-node` | **9944** |

单节点是本地开发及配合 `vibly-coordinator` 和 `vibly-indexer` 进行 E2E 测试的主要目标。

## Pallet 一览

### 共享（平行链 + 单节点）

| Pallet | 说明 |
|---|---|
| `pallet-identity-core` | 根身份、恢复账户、委托密钥、内容指针、外部传输绑定 |
| `pallet-payment-intent` | 基于原生资产（asset\_id=0）的支付意图，支持直接结算和持有结算 |

### 仅限单节点

| Pallet | 说明 |
|---|---|
| `pallet-onboarding-distribution` | 代理注册、注册商分配 |
| `pallet-agent-staking` | 代理质押绑定、解绑、释放阻塞机制 |
| `pallet-membership`（GuardianMembership） | 守护者成员管理；单个守护者成员即可暂停提案 |
| `pallet-collective`（GuardianCollective） | 守护者集体；2/3 多数可取消或恢复暂停 |
| `pallet-vibly-emergency` | 守护者成员或集体发起的紧急暂停/恢复/取消接口 |

> 单节点运行时不包含 OpenGov（`pallet_referenda`、`ConvictionVoting`、Treasury）。Vibly 的提案/投票/审核流程以协调器侧领域事件建模；只有最终的支付/惩罚/暂停事实才记录在链上。

## 前置条件

- Rust 工具链（参见 `rust-toolchain.toml`）
- `wasm32-unknown-unknown` target（rustup 自动安装）
- [Zombienet CLI](https://github.com/paritytech/zombienet)（用于多节点测试）：`npm install -g @zombienet/cli && zombienet setup polkadot`

## 构建

```bash
# 格式检查与 Lint
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

# 单元测试
cargo test --workspace --exclude vibly-chain-node -j1

# 构建平行链 Collator
cargo build --release -p vibly-chain-node

# 构建单节点（本地开发和 E2E）
cargo build --release -p vibly-solo-node
```

## 单节点：本地开发

```bash
cargo build --release -p vibly-solo-node
./target/release/vibly-solo-node --dev --tmp
# WebSocket: ws://127.0.0.1:9944
# Polkadot.js Apps: https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944
```

开启外部 RPC 访问（Docker 索引器所需）：

```bash
./target/release/vibly-solo-node --dev --tmp --rpc-external --rpc-cors all
```

配合协调器使用：

```bash
cd ../vibly-coordinator && pnpm dev
```

## 平行链：本地网络（Zombienet）

```bash
./scripts/dev/build.sh
./scripts/dev/zombienet-local.sh
```

生成开发链规格：

```bash
./scripts/dev/chain-spec.sh
```

## Paseo 测试网部署

```bash
./scripts/paseo/build-artifacts.sh
# 上传和 Collator 配置参见 scripts/paseo/README.md
```

## 仓库结构

| 目录 | 内容 |
|---|---|
| `node/` | 平行链 Collator CLI、链规格、RPC、服务 |
| `runtime/` | 平行链运行时（身份 + 支付） |
| `solo-node/` | 单节点 CLI |
| `solo-runtime/` | 单节点运行时（身份 + 支付 + 代理质押 + 守护紧急机制） |
| `pallets/identity-core/` | 身份状态机 Pallet |
| `pallets/payment-intent/` | 支付意图状态机 Pallet |
| `pallets/agent-staking/` | 代理质押绑定与释放阻塞 Pallet |
| `pallets/onboarding-distribution/` | 代理注册与注册商分配 Pallet |
| `primitives/` | 共享 SCALE 类型 |
| `integration-tests/` | Zombienet 本地网络测试 |
| `scripts/dev/` | 本地构建与链规格工具 |
| `scripts/paseo/` | 测试网产物与 Collator 工具 |

## 贡献

参见 [CONTRIBUTING.md](CONTRIBUTING.md)。安全问题请通过 [SECURITY.md](SECURITY.md) 报告。
