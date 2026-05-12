# Pallets / 自定义 Pallet 模块

This directory contains the custom vibly-chain FRAME pallets built with Substrate's
FRAME v2 `frame::pallet` macro.

本目录包含 vibly-chain 所有自定义 FRAME pallet 模块，基于 Substrate
`frame::pallet` 宏构建（FRAME v2 实验性 API）。

## Overview / 概览

| Pallet | Crate | Summary / 简介 |
|--------|-------|----------------|
| `identity-core` | `pallet-identity-core` | Root identity registry, delegated keys, content pointers, transport bindings / 根身份注册表、委托密钥、内容指针、传输绑定 |
| `agent-staking` | `pallet-agent-staking` | Agent stake holds, unbonding, and release blocking / Agent 质押持仓、解绑延迟与释放阻止 |
| `onboarding-distribution` | `pallet-onboarding-distribution` | Trusted onboarding, agent registration, and distribution flows / 可信入网、代理注册与分发流程 |
| `payment-intent` | `pallet-payment-intent` | Native-asset payment intents with direct or hold-based settlement / 原生资产支付意向，支持直接结算或持仓结算 |
| `vibly-emergency` | `pallet-vibly-emergency` | Guardian-controlled emergency status registry for scopes / Guardian 控制的应急状态注册表 |

## `identity-core`

Stores and governs root identities on-chain. Each identity has an owner account,
an optional recovery account, delegated capability keys, active content pointers,
and external transport bindings. Other pallets consume identity information
through `IdentityAccess` instead of reading identity storage directly.

在链上存储和管理根身份。其他 pallet 通过 `IdentityAccess` trait 访问身份信息，
不直接读取身份存储。

Key capabilities include `CAP_MANAGE_POINTERS`, `CAP_MANAGE_TRANSPORTS`,
`CAP_MANAGE_PAYMENT`, `CAP_ADMIN`, and `CAP_REGISTER_AGENT`.

## `agent-staking`

Records stake ledgers for agents registered under an identity.

- Agent existence is checked against `pallet-onboarding-distribution::AgentRegistrations`.
- Authorization is delegated to `pallet-identity-core::IdentityAccess`.
- Stake funds are held via `fungible::hold::Mutate` using `HoldReason::AgentStake`.
- Root owners and authorized agent registrar/operator accounts can bond, request
  unbond, and cancel unbond.
- Coordinator authority can block and clear release while off-chain public
  obligations remain unsettled.

主要调用：

| Call | Authority / 调用权限 | Description / 说明 |
|---|---|---|
| `bond_agent` | Identity root or agent registrar/operator | Hold active stake for a registered agent / 为已注册 agent 锁定质押 |
| `request_unbond` | Identity root or agent registrar/operator | Move active stake into unbonding / 将活跃质押转入解绑期 |
| `cancel_unbond` | Identity root or agent registrar/operator | Return unbonding stake to active / 取消解绑 |
| `block_release` | Coordinator authority / root | Block release during unfinished public duties / 未完成公共义务时阻止释放 |
| `clear_release_block` | Coordinator authority / root | Clear a release block / 清除释放阻止 |
| `release_unbond` | Funding account | Release unlocked, unblocked stake / 释放到期且未阻止的质押 |

事件包括 `AgentStakeBonded`, `AgentStakeUnbondRequested`,
`AgentStakeUnbondCancelled`, `AgentStakeReleaseBlocked`,
`AgentStakeReleaseCleared`, and `AgentStakeReleased`。

## `onboarding-distribution`

Handles trusted local onboarding, distribution limits, EVM/DOT conversion issuance,
root rotation, and agent registration. It records `AgentRegistrations`, which are
consumed by `agent-staking` as the on-chain source of registered agent existence.

## `payment-intent`

Records identity-backed payment intents for the native asset. Direct settlement
transfers immediately, while hold settlement reserves funds until claim or refund.
Identity authorization is delegated to `IdentityAccess`.

## `vibly-emergency`

Records emergency state (`Active`, `Paused`, `Cancelled`) for scopes such as
proposals, reward batches, settlement batches, or global operations. It does not
transfer funds or execute tasks; off-chain coordinators consume its state.

## Running Tests / 运行测试

```bash
cargo test -p pallet-identity-core
cargo test -p pallet-agent-staking
cargo test -p pallet-onboarding-distribution
cargo test -p pallet-payment-intent
cargo test -p pallet-vibly-emergency
```

## Adding a New Pallet / 添加新 Pallet

1. Create `pallets/<name>/Cargo.toml` and `src/lib.rs`.
2. Add it to workspace `Cargo.toml` under `[workspace.members]`.
3. Wire it into the relevant runtime config and runtime pallet list.
4. Add focused pallet tests and update indexer/coordinator consumers when the
   pallet emits production-facing events.
