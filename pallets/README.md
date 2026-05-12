# Pallets / 自定义 Pallet 模块

This directory contains the custom vibly-chain FRAME pallets built with Substrate's
[FRAME v2 `frame::pallet` macro](https://docs.substrate.io/reference/frame-macros/).

本目录包含 vibly-chain 所有自定义 FRAME pallet 模块，基于 Substrate
`frame::pallet` 宏构建（FRAME v2 实验性 API）。

---

## Overview / 概览

| Pallet | Crate | Summary / 简介 |
|--------|-------|----------------|
| [`identity-core`](#identity-core) | `pallet-identity-core` | Root identity registry, delegated keys, content pointers, transport bindings / 根身份注册表、委托密钥、内容指针、传输绑定 |
| [`payment-intent`](#payment-intent) | `pallet-payment-intent` | Native-asset payment intents with direct or hold-based settlement / 原生资产支付意向，支持直接结算或持仓结算 |
| [`vibly-emergency`](#vibly-emergency) | `pallet-vibly-emergency` | Guardian-controlled emergency status registry for scopes / Guardian 控制的应急状态注册表 |

---

## `identity-core`

**Crate:** `pallet-identity-core`

### Purpose / 用途

Stores and governs root identities on-chain. Each identity has an owner account,
an optional recovery account, a set of delegated capability keys, up to four active
content pointers (profile, agent registry, auth registry, relation policy), and
external transport bindings (e.g. email, DID). Other pallets consume identity
information exclusively through the `IdentityAccess` trait — they never read
identity storage directly.

在链上存储和管理根身份。每个身份包含一个所有者账户、一个可选的恢复账户、一组委托
能力密钥、最多四个活跃内容指针（个人资料、代理注册表、授权注册表、关系策略），以及
外部传输绑定（如邮件、DID）。其他 pallet 仅通过 `IdentityAccess` trait 访问身份信息，
不直接读取身份存储。

### Storage / 存储项

| Storage Item | Description / 说明 |
|---|---|
| `Identities` | Root identity records keyed by `IdentityId` / 以 `IdentityId` 为键的根身份记录 |
| `AuthorizedKeys` | Delegated key records keyed by `KeyId` / 以 `KeyId` 为键的委托密钥记录 |
| `AuthorizedKeyIdByAccount` | Reverse lookup `(identity_id, account) → KeyId` / 反向索引 |
| `TransportBindings` | External transport binding records keyed by `TransportBindingId` / 外部传输绑定记录 |
| `TransportBindingByIdentityAndLocator` | Uniqueness index for `(identity, transport, account)` / 唯一性索引 |
| `NextIdentitySequence` | Monotonic counter for deriving identity IDs / 单调递增序列，用于派生身份 ID |
| `NextTransportSequence` | Monotonic counter for deriving transport binding IDs / 单调递增序列，用于派生传输绑定 ID |

### Dispatchable Calls / 可调用方法

| Index | Call | Authority / 调用权限 | Description / 说明 |
|---|---|---|---|
| 0 | `register_identity` | Any signed / 任意签名账户 | Register a new root identity / 注册新根身份 |
| 1 | `rotate_owner_key` | Owner or Recovery / 所有者或恢复账户 | Rotate the owner account / 轮换所有者账户 |
| 2 | `set_recovery_key` | Owner or Recovery / 所有者或恢复账户 | Set or clear the recovery account / 设置或清除恢复账户 |
| 3 | `add_key` | Owner or `CAP_MANAGE_POINTERS` / 所有者或具备指针管理能力的密钥 | Add a delegated key with capability bits / 添加带能力位的委托密钥 |
| 4 | `revoke_key` | Owner or `CAP_MANAGE_POINTERS` / 同上 | Revoke a delegated key / 撤销委托密钥 |
| 5 | `set_active_profile` | Owner or `CAP_MANAGE_POINTERS` | Update profile content pointer / 更新个人资料内容指针 |
| 6 | `set_active_agent_registry` | Owner or `CAP_MANAGE_POINTERS` | Update agent registry pointer / 更新代理注册表指针 |
| 7 | `set_active_auth_registry` | Owner or `CAP_MANAGE_POINTERS` | Update auth registry pointer / 更新授权注册表指针 |
| 8 | `set_active_relation_policy` | Owner or `CAP_MANAGE_POINTERS` | Update relation policy pointer / 更新关系策略指针 |
| 9 | `bind_transport` | Owner or `CAP_MANAGE_TRANSPORTS` | Create a pending transport binding / 创建待验证的传输绑定 |
| 10 | `verify_transport` | Owner or `CAP_MANAGE_TRANSPORTS` | Verify a pending transport binding / 验证传输绑定 |
| 11 | `revoke_transport` | Owner or `CAP_MANAGE_TRANSPORTS` | Revoke a transport binding / 撤销传输绑定 |
| 12 | `freeze_identity` | Owner or Recovery / 所有者或恢复账户 | Freeze an active identity / 冻结活跃身份 |
| 13 | `unfreeze_identity` | Owner or Recovery | Reactivate a frozen identity / 解冻已冻结身份 |
| 14 | `disable_identity` | Owner or Recovery | Permanently disable an identity / 永久禁用身份 |

### Capability Bits / 能力位

Delegated keys carry a `CapabilityMask` bitmask. The pallet enforces four
built-in scopes:

委托密钥携带 `CapabilityMask` 位掩码，pallet 强制执行四个内置权限范围：

| Constant | Scope | Allowed operations / 允许操作 |
|---|---|---|
| `CAP_ADMIN` | Admin | Owner/recovery lifecycle / 所有者/恢复账户生命周期 |
| `CAP_MANAGE_POINTERS` | Pointer Manager | Content pointers + key management / 内容指针 + 密钥管理 |
| `CAP_MANAGE_TRANSPORTS` | Transport Manager | Transport bindings / 传输绑定 |
| `CAP_MANAGE_PAYMENT` | Payment Manager | Cross-pallet payment authorization / 跨 pallet 支付授权 |

### Events / 事件

`IdentityRegistered`, `OwnerKeyRotated`, `RecoveryKeySet`, `IdentityKeyAdded`,
`IdentityKeyRevoked`, `ActiveProfileSet`, `ActiveAgentRegistrySet`,
`ActiveAuthRegistrySet`, `ActiveRelationPolicySet`, `TransportBound`,
`TransportVerified`, `TransportRevoked`, `IdentityFrozen`, `IdentityUnfrozen`,
`IdentityDisabled`.

### Errors / 错误码

Key errors include / 主要错误包括：`IdentityNotFound`, `Unauthorized`,
`KeyAlreadyExists`, `KeyExpired`, `KeyRevoked`, `TransportBindingAlreadyExists`,
`TransportVerificationFailed`, `InvalidState`, `Overflow`.

---

## `payment-intent`

**Crate:** `pallet-payment-intent`

### Purpose / 用途

Records identity-backed payment intents for the native asset (`asset_id = 0`).
Supports two settlement modes:

- **Direct** — funds transfer immediately from the funding account to the payee owner when `fund_payment_intent` is called.
- **Hold** — funds are reserved (held) on-chain until the payee calls `claim_payment_intent` or the payer calls `refund_payment_intent`.

Identity authorization is delegated to `pallet-identity-core` via the
`IdentityAccess` trait — this pallet never reads identity storage directly.

为原生资产（`asset_id = 0`）记录基于身份的支付意向，支持两种结算模式：

- **直接结算（Direct）**：调用 `fund_payment_intent` 时资金立即从付款账户转入收款方所有者。
- **持仓结算（Hold）**：资金在链上被持仓保留，直到收款方调用 `claim_payment_intent` 领取，或付款方调用 `refund_payment_intent` 退款。

身份授权通过 `IdentityAccess` trait 委托给 `pallet-identity-core`，本 pallet 不直接读取身份存储。

### Storage / 存储项

| Storage Item | Description / 说明 |
|---|---|
| `PaymentIntents` | Intent records keyed by `PaymentIntentId` / 以 `PaymentIntentId` 为键的意向记录 |
| `IntentFundingAccounts` | Funding account for hold-settlement intents / 持仓结算的资金账户 |
| `PaymentIntentsByPayer` | Sparse index by payer identity / 按付款方身份的稀疏索引 |
| `PaymentIntentsByPayee` | Sparse index by payee identity / 按收款方身份的稀疏索引 |

### Payment Intent State Machine / 支付意向状态机

```
Requested ──fund──▶ Claimed (Direct)
Requested ──fund──▶ Funded ──claim──▶ Claimed
                           └──refund──▶ Refunded
Requested ──cancel──▶ Cancelled
Requested ──expire──▶ Expired  (after expires_at, any signer)
```

### Dispatchable Calls / 可调用方法

| Index | Call | Authority / 调用权限 | Description / 说明 |
|---|---|---|---|
| 0 | `create_payment_intent` | `CAP_MANAGE_PAYMENT` for payer / 付款方支付管理能力 | Create a requested intent / 创建支付意向 |
| 1 | `fund_payment_intent` | `CAP_MANAGE_PAYMENT` for payer | Fund according to settlement mode / 按结算模式注资 |
| 2 | `claim_payment_intent` | `CAP_MANAGE_PAYMENT` for payee / 收款方 | Claim hold-settled funds / 领取持仓资金 |
| 3 | `refund_payment_intent` | `CAP_MANAGE_PAYMENT` for payer | Release held funds back to payer / 退还持仓资金 |
| 4 | `cancel_payment_intent` | `CAP_MANAGE_PAYMENT` for payer | Cancel before funding / 注资前取消 |
| 5 | `expire_payment_intent` | Any signed / 任意签名账户 | Mark expired after `expires_at` / 到期后标记过期 |

### Events / 事件

`PaymentIntentCreated`, `PaymentIntentFunded`, `PaymentIntentClaimed`,
`PaymentIntentRefunded`, `PaymentIntentCancelled`, `PaymentIntentExpired`.

### Errors / 错误码

`IntentAlreadyExists`, `IntentNotFound`, `InvalidState`, `Unauthorized`,
`InvalidAmount`, `InvalidAsset`, `InsufficientBalance`, `FundingUnavailable`,
`AlreadyExpired`, `NotYetExpired`.

---

## `vibly-emergency`

**Crate:** `pallet-vibly-emergency`

### Purpose / 用途

A pure status registry that records the emergency state (`Active`, `Paused`,
`Cancelled`) of named scopes such as governance proposals, reward batches, and
settlement batches. It does **not** transfer funds, trigger governance votes, or
execute tasks — it is consumed by off-chain coordinators and other pallets via
the `ensure_active` / `is_paused` / `is_cancelled` helpers.

纯状态注册表，记录命名范围（如治理提案、奖励批次、结算批次）的应急状态
（`Active`、`Paused`、`Cancelled`）。本 pallet **不**转移资金、触发治理投票或执行任务，
由链下协调器和其他 pallet 通过 `ensure_active` / `is_paused` / `is_cancelled` 辅助函数消费。

### Guardian Model (Scheme B) / Guardian 模型（方案 B）

| Action / 操作 | Origin / 来源 | Notes / 说明 |
|---|---|---|
| `pause` | `PauseOrigin` — any single Guardian member / 任意 Guardian 成员 | `Active → Paused` or refresh pause record / 刷新暂停记录 |
| `resume` | `ResumeOrigin` — Guardian collective m/n | `Paused → Active` / 恢复活跃 |
| `cancel` | `CancelOrigin` — Guardian collective m/n | `Active/Paused → Cancelled` (irreversible / 不可逆) |

### Scopes / 范围类型

```rust
pub enum EmergencyScope {
    Global,                    // Nuclear option — entire chain / 全链核选项
    Proposal(ScopeId),         // Single governance proposal / 单个治理提案
    RewardBatch(ScopeId),      // Reward distribution batch / 奖励分发批次
    SettlementBatch(ScopeId),  // Settlement batch / 结算批次
}
```

Absent storage entry ≡ `Active`. Only non-active scopes consume storage space.

存储中无条目等价于 `Active`，只有非活跃状态才占用存储空间。

### Storage / 存储项

| Storage Item | Description / 说明 |
|---|---|
| `StatusByScope` | Emergency status per scope, default `Active` / 每个范围的应急状态，默认 `Active` |
| `LastPauseRecord` | Most recent pause record `{ by, reason_hash }` / 最近一次暂停记录 |

### Runtime Helpers / 运行时辅助函数

```rust
// Returns Ok(()) if scope is Active, otherwise DispatchError.
// 若 scope 为 Active 则返回 Ok(()), 否则返回 DispatchError。
Pallet::<T>::ensure_active(scope)?;

Pallet::<T>::is_paused(scope);    // bool
Pallet::<T>::is_cancelled(scope); // bool
```

### Events / 事件

`Paused { scope, by, reason_hash }`, `Resumed { scope, reason_hash }`,
`Cancelled { scope, reason_hash }`.

### Errors / 错误码

`AlreadyCancelled`, `AlreadyActive`, `NotPaused`, `InvalidTransition`.

---

## Running Tests / 运行测试

```bash
# Individual pallets / 单独 pallet
cargo test -p pallet-identity-core
cargo test -p pallet-payment-intent
cargo test -p pallet-vibly-emergency

# All pallets at once / 一次运行所有
cargo test -p pallet-identity-core -p pallet-payment-intent -p pallet-vibly-emergency
```

## Benchmarks / 基准测试

Each pallet ships `src/benchmarking.rs` (except `vibly-emergency` which has none yet).
Run with a benchmark-enabled runtime build:

每个 pallet（除 `vibly-emergency` 外）均附带 `src/benchmarking.rs`。
需要启用了 benchmark feature 的 runtime 构建才能运行：

```bash
cargo build --release --features runtime-benchmarks
```

## Adding a New Pallet / 添加新 Pallet

1. Create `pallets/<name>/Cargo.toml` and `src/lib.rs`.  
   创建 `pallets/<name>/Cargo.toml` 与 `src/lib.rs`。
2. Add to workspace `Cargo.toml` under `[workspace.members]`.  
   在 workspace `Cargo.toml` 的 `[workspace.members]` 中注册。
3. Wire into `runtime/src/lib.rs` via `impl pallet_<name>::Config for Runtime`.  
   在 `runtime/src/lib.rs` 中通过 `impl pallet_<name>::Config for Runtime` 接入运行时。
4. Declare all `schema.response[200]` equivalents (storage types / events) before merging.  
   合并前声明所有存储类型和事件。
