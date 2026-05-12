# pallet-identity-core（身份核心模块）

> English documentation: [README.md](README.md)

本模块是 vibly-chain 的链上身份存储与治理核心，是所有者账户、可选恢复账户、委托能力密钥、活跃内容指针以及外部传输绑定的唯一权威来源。其他需要身份信息的模块均通过 `IdentityAccess` trait 进行消费，而不直接读取本模块的存储。

---

## 目录

1. [概览](#概览)
2. [架构](#架构)
3. [配置](#配置)
4. [存储项](#存储项)
5. [身份生命周期](#身份生命周期)
6. [能力模型](#能力模型)
7. [可调用方法](#可调用方法)
8. [事件](#事件)
9. [错误码](#错误码)
10. [集成（IdentityAccess trait）](#集成identityaccess-trait)
11. [测试](#测试)
12. [基准测试](#基准测试)

---

## 概览

每个**根身份**由一个确定性的 `IdentityId`（基于模块内单调序列哈希派生的 `H256`）标识。身份包含以下字段：

| 字段 | 说明 |
|---|---|
| `owner` | 主 `AccountId`；对该身份拥有完全权限 |
| `recovery` | 可选 `AccountId`；可执行所有者/恢复账户的生命周期操作 |
| `active_profile` | 可选 `ContentRef`，指向链下个人资料文档 |
| `active_agent_registry` | 可选 `ContentRef`，指向代理注册表文档 |
| `active_auth_registry` | 可选 `ContentRef`，指向授权注册表文档 |
| `active_relation_policy` | 可选 `ContentRef`，指向关系策略文档 |
| `status` | `Active`（活跃）\| `Frozen`（冻结）\| `Disabled`（永久禁用） |
| `nonce` | 单调突变计数器；每次状态变更时递增 |
| `created_at` / `updated_at` | 来自 `TimeProvider` 的毫秒时间戳 |

---

## 架构

```
             ┌──────────────────────────────────────────────────────┐
             │               pallet-identity-core                    │
             │                                                        │
             │  Identities  AuthorizedKeys  TransportBindings        │
             │       │             │               │                  │
             │       └─────────────┴───────────────┘                 │
             │                    ▲                                   │
             │          IdentityAccess trait                         │
             └──────────────────────────────────────────────────────┘
                        ▲                    ▲
               pallet-payment-intent    （未来其他模块）
```

其他模块调用 `T::IdentityProvider::ensure_can_manage_payment(identity, account)?` 等方法，而不直接查询 `Identities` 存储，使身份模型作为单一可插拔依赖存在。

---

## 配置

```rust
pub trait Config: frame_system::Config {
    /// 可调用方法的权重提供者
    type WeightInfo: WeightInfo;

    /// 返回毫秒级时间戳的时间提供者
    type TimeProvider: Time<Moment = u64>;

    /// 内容 CID 的最大字节长度
    #[pallet::constant]
    type MaxCidLen: Get<u32>;

    /// 内容 URI 的最大字节长度
    #[pallet::constant]
    type MaxUriLen: Get<u32>;

    /// 外部传输账户定位符的最大字节长度
    #[pallet::constant]
    type MaxTransportAccountLen: Get<u32>;
}
```

典型的运行时接入示例：

```rust
impl pallet_identity_core::Config for Runtime {
    type WeightInfo    = pallet_identity_core::weights::SubstrateWeight<Runtime>;
    type TimeProvider  = Timestamp;
    type MaxCidLen     = ConstU32<128>;
    type MaxUriLen     = ConstU32<256>;
    type MaxTransportAccountLen = ConstU32<128>;
}
```

---

## 存储项

| 存储项 | 键 | 值 | 说明 |
|---|---|---|---|
| `Identities` | `IdentityId` | `RootIdentity` | 根身份记录 |
| `AuthorizedKeys` | `KeyId` | `AuthorizedKeyRecord` | 委托密钥记录 |
| `AuthorizedKeyIdByAccount` | `(IdentityId, AccountId)` | `KeyId` | 账户到密钥 ID 的反向索引 |
| `TransportBindings` | `TransportBindingId` | `TransportBinding` | 外部传输绑定记录 |
| `TransportBindingByIdentityAndLocator` | `Hash256(identity, transport, account)` | `TransportBindingId` | 传输定位符唯一性保护 |
| `NextIdentitySequence` | — | `u64` | 身份 ID 派生用单调计数器 |
| `NextTransportSequence` | — | `u64` | 传输绑定 ID 派生用单调计数器 |

### ID 派生机制

身份 ID 和传输绑定 ID **不是**顺序整数，而是带有域分隔前缀的 `BlakeTwo256` 哈希：

```
IdentityId         = BlakeTwo256( ("vibly/identity",  seq) )
TransportBindingId = BlakeTwo256( ("vibly/transport", seq) )
KeyId              = BlakeTwo256( ("vibly/key", identity_id, account) )
```

这使 ID 不可预测，避免了枚举攻击。

---

## 身份生命周期

```
                register_identity
                       │
                       ▼
                    Active（活跃）
                    ┌────┐
                    │    │  ← 委托操作（指针、传输、密钥）
                    │    │  ← 所有者 / 恢复账户操作
                    └────┘
              freeze_identity│
                    ▼
                 Frozen（冻结）        ← 仅允许所有者/恢复操作
              unfreeze_identity│
                    ▲
                    │
              disable_identity（从 Active 或 Frozen）
                    ▼
                Disabled（永久禁用）   ← 不可变，不接受任何操作
```

- **冻结**：委托指针、传输和支付管理操作被拒绝。所有者和恢复账户仍可轮换所有者密钥、设置/清除恢复账户、解冻或禁用身份。
- **禁用**：身份记录保留以供审计，不再接受任何突变。

---

## 能力模型

委托密钥携带 `CapabilityMask` 位掩码，模块强制执行四个内置访问范围：

| 常量 | 位 | 允许操作 |
|---|---|---|
| `CAP_ADMIN` | `0x01` | 所有者/恢复生命周期（轮换所有者、设置恢复账户、冻结、解冻、禁用） |
| `CAP_MANAGE_POINTERS` | `0x02` | 内容指针突变；添加/撤销密钥也需要此能力 |
| `CAP_MANAGE_TRANSPORTS` | `0x04` | 创建和撤销传输绑定 |
| `CAP_MANAGE_PAYMENT` | `0x08` | 跨模块支付授权（`IdentityAccess::ensure_can_manage_payment`） |

**所有者**账户在所有范围内始终拥有完全权限。**恢复**账户仅限于 `OwnerOrRecovery` 范围的操作（生命周期管理）。委托密钥可持有四个位的任意组合。

### 密钥记录字段

```rust
pub struct AuthorizedKeyRecord<AccountId> {
    pub key_id:          KeyId,
    pub identity_id:     IdentityId,
    pub account:         AccountId,
    pub purpose:         KeyPurpose,    // 如 Finance, Signing 等
    pub capability_mask: CapabilityMask,
    pub expires_at:      Option<u64>,   // 毫秒；None 表示无过期
    pub revoked_at:      Option<u64>,
    pub created_at:      u64,
}
```

---

## 可调用方法

### 身份生命周期

| 索引 | 方法 | 最低权限 | 说明 |
|---|---|---|---|
| 0 | `register_identity(recovery, active_profile, ...)` | 任意签名账户 | 以签名者为所有者创建新根身份 |
| 1 | `rotate_owner_key(identity_id, new_owner)` | 所有者**或**恢复账户 | 替换所有者账户 |
| 2 | `set_recovery_key(identity_id, new_recovery)` | 所有者**或**恢复账户 | 设置或清除恢复账户 |
| 12 | `freeze_identity(identity_id)` | 所有者**或**恢复账户 | 暂停委托操作 |
| 13 | `unfreeze_identity(identity_id)` | 所有者**或**恢复账户 | 重新激活已冻结身份 |
| 14 | `disable_identity(identity_id)` | 所有者**或**恢复账户 | 永久禁用（不可逆） |

### 委托密钥管理

| 索引 | 方法 | 最低权限 | 说明 |
|---|---|---|---|
| 3 | `add_key(identity_id, account, purpose, capability_mask, expires_at)` | 所有者**或** `CAP_MANAGE_POINTERS` | 添加委托密钥；拒绝 `Owner`/`Recovery` purpose |
| 4 | `revoke_key(identity_id, key_id)` | 所有者**或** `CAP_MANAGE_POINTERS` | 删除委托密钥及其反向索引条目 |

### 内容指针

所有指针方法共享相同的权限要求：所有者或持有 `CAP_MANAGE_POINTERS` 的委托密钥。身份必须处于 `Active` 状态。

| 索引 | 方法 | 说明 |
|---|---|---|
| 5 | `set_active_profile(identity_id, profile)` | 设置或清除个人资料指针 |
| 6 | `set_active_agent_registry(identity_id, registry)` | 设置或清除代理注册表指针 |
| 7 | `set_active_auth_registry(identity_id, registry)` | 设置或清除授权注册表指针 |
| 8 | `set_active_relation_policy(identity_id, policy)` | 设置或清除关系策略指针 |

### 传输绑定

| 索引 | 方法 | 最低权限 | 说明 |
|---|---|---|---|
| 9 | `bind_transport(identity_id, transport, account, proof_ref)` | 所有者**或** `CAP_MANAGE_TRANSPORTS` | 创建 `Pending` 状态的传输绑定；定位符必须唯一 |
| 10 | `verify_transport(identity_id, binding_id, proof_ref)` | 仅所有者**或**恢复账户 | 将绑定升级为 `Verified`；此操作需要身份控制权 |
| 11 | `revoke_transport(identity_id, binding_id)` | 所有者**或** `CAP_MANAGE_TRANSPORTS` | 将绑定标记为 `Revoked`（记录保留以供审计） |

> **注意：** `verify_transport` 特别要求所有者或恢复账户，因为验证动作断言的是对身份本身的控制权，而非委托的传输管理权限。

---

## 事件

```rust
pub enum Event<T: Config> {
    IdentityRegistered       { identity_id, owner },            // 新根身份已注册
    OwnerKeyRotated          { identity_id, old_owner, new_owner }, // 所有者账户已轮换
    RecoveryKeySet           { identity_id },                   // 恢复账户已设置/清除
    IdentityKeyAdded         { identity_id, key_id, purpose },  // 委托密钥已添加
    IdentityKeyRevoked       { identity_id, key_id },           // 委托密钥已撤销
    ActiveProfileSet         { identity_id },                   // 个人资料指针已更新
    ActiveAgentRegistrySet   { identity_id },                   // 代理注册表指针已更新
    ActiveAuthRegistrySet    { identity_id },                   // 授权注册表指针已更新
    ActiveRelationPolicySet  { identity_id },                   // 关系策略指针已更新
    TransportBound           { identity_id, binding_id, transport }, // 传输绑定已创建
    TransportVerified        { identity_id, binding_id },       // 传输绑定已验证
    TransportRevoked         { identity_id, binding_id },       // 传输绑定已撤销
    IdentityFrozen           { identity_id },                   // 身份已冻结
    IdentityUnfrozen         { identity_id },                   // 身份已解冻
    IdentityDisabled         { identity_id },                   // 身份已永久禁用
}
```

---

## 错误码

| 错误 | 含义 |
|---|---|
| `IdentityAlreadyExists` | 生成的身份 ID 发生碰撞（实际中几乎不会发生） |
| `IdentityNotFound` | 给定 `IdentityId` 无对应记录 |
| `InvalidState` | 当前身份或绑定状态不允许此操作 |
| `AlreadyFrozen` / `NotFrozen` | 冻结/解冻状态保护 |
| `AlreadyDisabled` | 身份已永久禁用 |
| `Unauthorized` | 调用者缺少所有者权限、恢复账户权限或所需能力位 |
| `OwnerKeyRequired` | 该调用特别要求所有者密钥 |
| `RecoveryNotConfigured` | 未设置恢复账户 |
| `RecoveryNotAllowed` | 恢复账户无法执行此特定操作 |
| `KeyAlreadyExists` | 该账户的委托密钥已存在 |
| `KeyNotFound` | 给定 `KeyId` 无对应委托密钥 |
| `KeyInvalid` | 密钥会与所有者/恢复账户重复，或 purpose 为 `Owner`/`Recovery` |
| `KeyExpired` | 密钥的 `expires_at` 已过期 |
| `KeyRevoked` | 密钥已被撤销 |
| `PointerInvalid` | 内容引用结构无效 |
| `TransportBindingAlreadyExists` | 定位符 `(identity, transport, account)` 已绑定 |
| `TransportBindingNotFound` | 给定 `TransportBindingId` 无对应绑定 |
| `TransportVerificationFailed` | 证明验证失败 |
| `TransportNotAllowed` | 传输类型或账户不被允许 |
| `NonceInvalid` | 提供的 nonce 不匹配 |
| `Overflow` | 序列计数器溢出（理论上） |
| `InvalidInput` | 通用格式错误输入 |

---

## 集成（IdentityAccess trait）

其他模块在配置中声明 `type IdentityProvider: IdentityAccess<Self::AccountId>`，并调用：

```rust
// 验证身份存在
T::IdentityProvider::identity_exists(&identity_id);

// 要求调用者对该身份拥有支付管理权限
T::IdentityProvider::ensure_can_manage_payment(&identity_id, &who)?;

// 要求调用者对该身份拥有支付领取权限（收款方）
T::IdentityProvider::ensure_can_claim_payment(&identity_id, &who)?;

// 解析所有者账户用于资金转移
let owner = T::IdentityProvider::owner_account(&identity_id)?;
```

运行时接入示例：

```rust
impl pallet_payment_intent::Config for Runtime {
    type IdentityProvider = IdentityCore;
    // …
}
```

---

## 测试

```bash
cargo test -p pallet-identity-core
```

`src/tests.rs` 覆盖的主要测试场景：

- `register_and_manage_identity_works` — 完整注册、指针突变、密钥添加
- `transport_flow_works` — 绑定 → 验证 → 撤销传输绑定
- 所有者轮换和恢复密钥管理
- 冻结 / 解冻 / 禁用状态转换
- 带能力掩码的委托授权校验
- 委托密钥的过期和撤销

---

## 基准测试

`src/benchmarking.rs` 为所有可调用方法提供 FRAME 基准测试。需要启用了 `runtime-benchmarks` feature 的节点：

```bash
cargo build --release --features runtime-benchmarks
./target/release/vibly-node benchmark pallet \
  --pallet pallet_identity_core \
  --extrinsic "*" \
  --steps 50 --repeat 20 \
  --output pallets/identity-core/src/weights.rs
```
