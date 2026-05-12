# vibly-chain

vibly-chain 是基于 Polkadot SDK 构建的 Vibly 协调链。包含两个运行模式：

- **Parachain 模式**（`node/` + `runtime/`）：作为 Rococo/Paseo 的平行链运行，专注于身份根、内容指针、外部传输绑定和原生资产支付意图。
- **Solo-node 模式**（`solo-node/` + `solo-runtime/`）：独立开发链，内置 **Guardian 紧急治理**（pallet-membership + pallet-collective + pallet-vibly-emergency），用于本地支付意图和紧急暂停功能开发和测试。

`vibly-coordinator` 通过 REST/SSE API 消费链上事件，`vibly-indexer` 可索引链上状态变更。

## 运行时对比

| 特性 | Parachain runtime | Solo runtime |
|---|---|---|
| OpenGov（pallet_referenda）| ❌ | ❌ |
| ConvictionVoting | ❌ | ❌ |
| 身份核心（pallet-identity-core）| ✅ | ✅ |
| 支付意图（pallet-payment-intent）| ✅ | ✅ |
| Guardian 紧急治理（pallet-vibly-emergency）| ❌ | ✅ |
| 默认 RPC 端口 | 9988 | **9944** |

## Parachain 范畴（`runtime/`）

- `pallet-identity-core`：根身份、恢复账号、委托密钥、内容指针、外部传输绑定。
- `pallet-payment-intent`：原生资产（asset_id=0）支付意图，支持直接结算和锁定资金结算。
- `primitives/*`：SCALE 共享类型。

## Solo-node Guardian 紧急治理（`solo-runtime/`）

v0.2 solo-runtime 专注于 Guardian 紧急暂停机制：

- `pallet-identity-core`：根身份、恢复账号、委托密钥、内容指针、外部传输绑定。
- `pallet-payment-intent`：原生资产（asset_id=0）支付意图，支持直接结算和锁定资金结算。
- `pallet-membership`（GuardianMembership）：Guardian 成员管理，单个 Guardian 可暂停提案。
- `pallet-collective`（GuardianCollective）：Guardian 集体，2/3 多数可取消或恢复暂停状态。
- `pallet-vibly-emergency`：紧急暂停/恢复/取消接口，源于 Guardian 成员或集体 Origin。

> **注意：** OpenGov（pallet_referenda / ConvictionVoting / Treasury 等）已在 v0.2 中从 solo-runtime 移除。
> Vibly 的提案/投票/审查流程是 Coordinator 侧领域模型，链上仅记录最终支付/惩罚/暂停事实。

默认 WebSocket 端点：`ws://127.0.0.1:9944`

## 前置条件

- Rust toolchain（见 `rust-toolchain.toml`）
- `wasm32-unknown-unknown` target（由 rustup 自动安装）
- Zombienet CLI（多节点测试）：`npm install -g @zombienet/cli && zombienet setup polkadot`

## 常用命令

```bash
# 格式检查 & Lint
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

# 运行测试
cargo test --workspace --exclude vibly-chain-node -j1

# 构建 parachain node
cargo build --release -p vibly-chain-node

# 构建 solo-node（Guardian 开发链）
cargo build --release -p vibly-solo-node
```

## Solo-node 开发链（Guardian 本地测试）

```bash
# 构建并启动 solo-node（--dev 模式，自动清空状态）
cargo build --release -p vibly-solo-node
./target/release/vibly-solo-node --dev

# 默认 RPC：ws://127.0.0.1:9944
# 可用 Polkadot.js Apps 连接：https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944
```

启动后，配合 `vibly-coordinator` 即可处理支付意图和紧急暂停事件：

```bash
cd ../vibly-coordinator && pnpm dev
```

## Parachain 本地网络（Zombienet）

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
# 详见 scripts/paseo/README.md
```

## 仓库结构

| 目录 | 内容 |
|---|---|
| `node/` | Parachain collator CLI、链规格、RPC、服务 |
| `runtime/` | Parachain runtime（身份 + 支付）|
| `solo-node/` | Solo-node CLI |
| `solo-runtime/` | Solo runtime（Guardian 紧急治理）|
| `pallets/identity-core/` | 身份状态机 pallet |
| `pallets/payment-intent/` | 支付意图状态机 pallet |
| `primitives/` | 共享 SCALE 类型 |
| `integration-tests/` | Zombienet 本地网络测试 |
| `scripts/dev/` | 本地构建和链规格工具 |
| `scripts/paseo/` | 测试网构件和 collator 工具 |

## 贡献

详见 `CONTRIBUTING.md`。安全问题请通过 `SECURITY.md` 报告。
