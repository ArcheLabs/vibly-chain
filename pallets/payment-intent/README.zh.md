# pallet-payment-intent（支付意向模块）

> English documentation: [README.md](README.md)

本模块在 vibly-chain 上记录基于身份的原生资产支付意向，支持两种结算模式——**直接结算**（即时转账）和**持仓结算**（资金暂时保留，直到被领取或退款）。身份授权完全通过 `IdentityAccess` trait 委托给 `pallet-identity-core`。

---

## 目录

1. [概览](#概览)
2. [架构](#架构)
3. [配置](#配置)
4. [存储项](#存储项)
5. [状态机](#状态机)
6. [结算模式](#结算模式)
7. [可调用方法](#可调用方法)
8. [事件](#事件)
9. [错误码](#错误码)
10. [测试](#测试)
11. [基准测试](#基准测试)

---

## 概览

**支付意向**代表链上一方对另一方的正式支付义务记录。生命周期为：

1. **创建** — 付款方的授权代理记录意向，包含金额、结算模式、可选到期时间和操作描述符。
2. **注资** — 付款方的授权代理转移（或持仓）资金。
3. **领取 / 退款 / 取消 / 过期** — 意向在某个终态结束。

当前实现仅支持原生资产（`asset_id = 0`）。

### 意向记录字段

| 字段 | 说明 |
|---|---|
| `intent_id` | 调用者提供的 `H256` 标识符；必须全局唯一 |
| `payer` / `payee` | 各方的 `IdentityId` |
| `asset_id` | 资产标识符；必须为 `0`（原生资产） |
| `amount` | 非零的 `u128` 最小面额金额 |
| `action` | `{ namespace, action_code, payload_ref }` — 描述所支付的服务或工作 |
| `memo_ref` | 可选 `ContentRef`，指向链下备注文档 |
| `settlement_mode` | `Direct`（直接）或 `Hold`（持仓） |
| `expires_at` | 可选毫秒时间戳；`None` 表示永不过期 |
| `payer_nonce` | 预留给未来防重放保护使用 |
| `status` | 当前状态（见状态机） |
| `created_at` / `updated_at` | 毫秒时间戳 |

---

## 架构

```
pallet-payment-intent
        │
        ├── T::IdentityProvider（IdentityAccess）
        │       → pallet-identity-core
        │
        └── T::Currency（fungible::Mutate + fungible::hold::Mutate）
                → pallet-balances（原生资产）
```

本模块从不直接读取 `pallet-identity-core` 的存储，而是调用：

- `identity_exists` — 创建意向前
- `ensure_can_manage_payment` — 创建、注资、退款、取消（付款方侧）前
- `ensure_can_claim_payment` — 领取（收款方侧）前
- `owner_account` — 解析资金转账目标账户

---

## 配置

```rust
pub trait Config: frame_system::Config {
    /// 可调用方法的权重提供者
    type WeightInfo: WeightInfo;

    /// 返回毫秒级时间戳的时间提供者
    type TimeProvider: Time<Moment = u64>;

    /// 身份查找和授权提供者
    type IdentityProvider: IdentityAccess<Self::AccountId>;

    /// 用于直接转账和持仓的原生货币
    type Currency: Mutate<Self::AccountId, Balance = Amount>
        + HoldMutate<Self::AccountId, Balance = Amount, Reason = Self::RuntimeHoldReason>;

    /// 运行时范围的持仓原因枚举
    type RuntimeHoldReason: From<HoldReason>;

    /// 支付操作命名空间的最大字节长度
    #[pallet::constant]
    type MaxNamespaceLen: Get<u32>;

    /// 内容 CID 的最大字节长度
    #[pallet::constant]
    type MaxCidLen: Get<u32>;

    /// 内容 URI 的最大字节长度
    #[pallet::constant]
    type MaxUriLen: Get<u32>;
}
```

典型的运行时接入示例：

```rust
impl pallet_payment_intent::Config for Runtime {
    type WeightInfo        = pallet_payment_intent::weights::SubstrateWeight<Runtime>;
    type TimeProvider      = Timestamp;
    type IdentityProvider  = IdentityCore;
    type Currency          = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type MaxNamespaceLen   = ConstU32<64>;
    type MaxCidLen         = ConstU32<128>;
    type MaxUriLen         = ConstU32<256>;
}
```

---

## 存储项

| 存储项 | 键 | 值 | 说明 |
|---|---|---|---|
| `PaymentIntents` | `PaymentIntentId` | `PaymentIntent` | 完整意向记录 |
| `IntentFundingAccounts` | `PaymentIntentId` | `AccountId` | 持仓结算的资金账户；仅在状态为 `Funded` 时存在 |
| `PaymentIntentsByPayer` | `(IdentityId, PaymentIntentId)` | `()` | 按付款方身份列举意向的稀疏索引 |
| `PaymentIntentsByPayee` | `(IdentityId, PaymentIntentId)` | `()` | 按收款方身份列举意向的稀疏索引 |

### 持仓原因

```rust
pub enum HoldReason {
    PaymentIntent,  // 意向处于 `Funded` 状态时持有的原生余额
}
```

持仓通过 `fungible::hold::Mutate` 接口放置和释放，确保持有金额被专属保留，不可被花费。

---

## 状态机

```
                  create_payment_intent
                          │
                          ▼
                      Requested（已请求）
                     /    │    \
          fund(Direct)    │     cancel_payment_intent
               │          │              │
               ▼          │              ▼
            Claimed    fund(Hold)    Cancelled ◄─ 终态
          （已领取）        │
                          ▼
                        Funded（已持仓）
                       /      \
          claim_payment_intent  refund_payment_intent
                  │                      │
                  ▼                      ▼
          Claimed ◄─ 终态          Refunded ◄─ 终态
                                  （已退款）

          （从 Requested，超过 expires_at 后）
                  │
          expire_payment_intent
                  │
                  ▼
           Expired ◄─ 终态（已过期）
```

终态：`Claimed`、`Refunded`、`Cancelled`、`Expired`。

---

## 结算模式

### 直接结算（`SettlementMode::Direct`）

`fund_payment_intent` 立即调用 `Currency::transfer`，将资金从注资账户转移到收款方身份当前的 `owner_account`。意向在同一笔外部调用中直接变为 `Claimed`，不进行任何持仓，也不需要领取步骤。

```
fund(Direct)
  └─ transfer(注资账户 → 收款方所有者, 金额)
  └─ 状态：Requested → Claimed
```

### 持仓结算（`SettlementMode::Hold`）

`fund_payment_intent` 通过 `Currency::hold(HoldReason::PaymentIntent)` 在注资账户上持仓 `amount`，意向变为 `Funded`，注资账户被记录在 `IntentFundingAccounts` 中。

```
fund(Hold)
  └─ hold(注资账户, 金额)
  └─ 状态：Requested → Funded

claim_payment_intent
  └─ transfer_on_hold(注资账户 → 收款方所有者, 金额)
  └─ 状态：Funded → Claimed

refund_payment_intent
  └─ release(注资账户, 金额)
  └─ 状态：Funded → Refunded
```

---

## 可调用方法

| 索引 | 方法 | 权限 | 说明 |
|---|---|---|---|
| 0 | `create_payment_intent(intent_id, payer, payee, asset_id, amount, action, memo_ref, settlement_mode, expires_at)` | 付款方的 `CAP_MANAGE_PAYMENT` | 创建 `Requested` 状态意向；`asset_id` 必须为 `0`；`amount` 必须非零 |
| 1 | `fund_payment_intent(intent_id)` | 付款方的 `CAP_MANAGE_PAYMENT` | 按结算模式注资；意向必须为 `Requested` 且未过期 |
| 2 | `claim_payment_intent(intent_id, evidence_ref)` | 收款方的 `CAP_MANAGE_PAYMENT` | 将持仓资金转给收款方所有者；意向必须为 `Funded` |
| 3 | `refund_payment_intent(intent_id, evidence_ref)` | 付款方的 `CAP_MANAGE_PAYMENT` | 将持仓资金释放回注资账户；意向必须为 `Funded` |
| 4 | `cancel_payment_intent(intent_id)` | 付款方的 `CAP_MANAGE_PAYMENT` | 在任何资金流动前取消；意向必须为 `Requested` |
| 5 | `expire_payment_intent(intent_id)` | 任意签名账户 | 在 `expires_at` 到期后将 `Requested` 意向标记为 `Expired` |

### 权限说明

- 所有付款方侧调用通过 `T::IdentityProvider::ensure_can_manage_payment` 检查权限，领取通过 `ensure_can_claim_payment` 检查。
- `expire_payment_intent` 只需有效的签名来源，因为时间戳检查和状态转换是完全确定性的。
- `claim` 和 `refund` 上的 `evidence_ref` 参数为链下证据锚定的未来预留，当前被忽略。

---

## 事件

```rust
pub enum Event<T: Config> {
    PaymentIntentCreated {
        intent_id,
        payer: IdentityId,   // 付款方
        payee: IdentityId,   // 收款方
        asset_id: AssetId,
        amount: Amount,
        action: PaymentAction,
    },
    PaymentIntentFunded {
        intent_id,
        settlement_mode: SettlementMode, // 已按此模式注资
    },
    PaymentIntentClaimed   { intent_id }, // 资金已转给收款方
    PaymentIntentRefunded  { intent_id }, // 持仓资金已退还付款方
    PaymentIntentCancelled { intent_id }, // 意向已取消
    PaymentIntentExpired   { intent_id }, // 意向已过期
}
```

---

## 错误码

| 错误 | 含义 |
|---|---|
| `IntentAlreadyExists` | 提供的 `intent_id` 已被使用 |
| `IntentNotFound` | 给定 `intent_id` 无对应记录 |
| `InvalidState` | 当前状态不允许请求的转换 |
| `Unauthorized` | 调用者未获身份能力模型的授权 |
| `InvalidAmount` | 金额为零 |
| `InvalidAsset` | `asset_id` 不为 `0` |
| `InvalidAction` | 操作命名空间为空或无效 |
| `InvalidSettlementMode` | 结算模式不支持 |
| `FundingUnavailable` | 持仓结算的注资账户记录缺失 |
| `InsufficientBalance` | 注资账户余额不足以覆盖持仓或转账 |
| `AlreadyExpired` | 意向已过期，无法注资 |
| `NotYetExpired` | 尚未到达 `expires_at`，无法过期 |
| `ClaimNotAllowed` | 当前状态不允许领取转换 |
| `RefundNotAllowed` | 当前状态不允许退款转换 |
| `CancelNotAllowed` | 意向不处于 `Requested` 状态，无法取消 |
| `ExpireNotAllowed` | 意向不处于 `Requested` 状态，无法过期 |
| `EvidenceInvalid` | 证据内容引用无效 |
| `NonceInvalid` | Nonce 不匹配 |
| `Overflow` | 算术溢出 |
| `InvalidInput` | 通用格式错误输入 |

---

## 测试

```bash
cargo test -p pallet-payment-intent
```

`src/tests.rs` 覆盖的主要测试场景：

- `create_and_direct_fund_works` — 创建意向，使用直接模式注资，断言状态为 `Claimed`
- `hold_claim_and_refund_state_machine_works` — 完整的持仓周期：创建 → 注资 → 领取；以及创建 → 注资 → 退款
- 各调用的非授权调用者拒绝
- 过期强制：`expires_at` 后可过期，之前被拒绝
- 注资前取消
- 错误情况：重复 `intent_id`、金额为零、错误 `asset_id`

---

## 基准测试

`src/benchmarking.rs` 为所有可调用方法提供 FRAME 基准测试：

```bash
cargo build --release --features runtime-benchmarks
./target/release/vibly-node benchmark pallet \
  --pallet pallet_payment_intent \
  --extrinsic "*" \
  --steps 50 --repeat 20 \
  --output pallets/payment-intent/src/weights.rs
```
