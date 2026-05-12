# pallet-payment-intent

> 中文文档：[README.zh.md](README.zh.md)

A FRAME pallet that records identity-backed native-asset payment intents on the
vibly-chain. It supports two settlement modes — **Direct** (immediate fund transfer)
and **Hold** (funds reserved until claimed or refunded). Identity authorization is
fully delegated to `pallet-identity-core` through the `IdentityAccess` trait.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Configuration](#configuration)
4. [Storage](#storage)
5. [State Machine](#state-machine)
6. [Settlement Modes](#settlement-modes)
7. [Dispatchable Calls](#dispatchable-calls)
8. [Events](#events)
9. [Errors](#errors)
10. [Testing](#testing)
11. [Benchmarks](#benchmarks)

---

## Overview

A **payment intent** represents a formal on-chain record of a payment obligation
from one identity to another. The lifecycle is:

1. **Create** — the payer's authorized agent records the intent with an amount,
   settlement mode, optional expiry, and an action descriptor.
2. **Fund** — the payer's authorized agent moves (or holds) funds.
3. **Claim / Refund / Cancel / Expire** — intent settles in one of the terminal
   states.

Only the native asset (`asset_id = 0`) is supported in the current implementation.

### Intent record fields

| Field | Description |
|---|---|
| `intent_id` | Caller-supplied `H256` identifier; must be globally unique |
| `payer` / `payee` | `IdentityId` for each party |
| `asset_id` | Asset identifier; must be `0` (native) |
| `amount` | Non-zero `u128` amount in the smallest denomination |
| `action` | `{ namespace, action_code, payload_ref }` — describes the service or work being paid for |
| `memo_ref` | Optional `ContentRef` to an off-chain memo document |
| `settlement_mode` | `Direct` or `Hold` |
| `expires_at` | Optional millisecond timestamp; `None` = never expires |
| `payer_nonce` | Reserved for replay-protection future use |
| `status` | Current state (see State Machine) |
| `created_at` / `updated_at` | Millisecond timestamps |

---

## Architecture

```
pallet-payment-intent
        │
        ├── T::IdentityProvider (IdentityAccess)
        │       → pallet-identity-core
        │
        └── T::Currency (fungible::Mutate + fungible::hold::Mutate)
                → pallet-balances (native asset)
```

The pallet never reads `pallet-identity-core` storage directly. It calls:

- `identity_exists` — before creating an intent
- `ensure_can_manage_payment` — before create, fund, refund, cancel (payer side)
- `ensure_can_claim_payment` — before claim (payee side)
- `owner_account` — to resolve the destination account for fund transfers

---

## Configuration

```rust
pub trait Config: frame_system::Config {
    /// Weight provider for dispatchable calls.
    type WeightInfo: WeightInfo;

    /// Timestamp provider returning milliseconds since epoch.
    type TimeProvider: Time<Moment = u64>;

    /// Identity lookup and authorization provider.
    type IdentityProvider: IdentityAccess<Self::AccountId>;

    /// Native currency used for direct transfers and holds.
    type Currency: Mutate<Self::AccountId, Balance = Amount>
        + HoldMutate<Self::AccountId, Balance = Amount, Reason = Self::RuntimeHoldReason>;

    /// Runtime-wide hold reason enum.
    type RuntimeHoldReason: From<HoldReason>;

    /// Maximum byte length for payment action namespaces.
    #[pallet::constant]
    type MaxNamespaceLen: Get<u32>;

    /// Maximum byte length for content CIDs.
    #[pallet::constant]
    type MaxCidLen: Get<u32>;

    /// Maximum byte length for content URIs.
    #[pallet::constant]
    type MaxUriLen: Get<u32>;
}
```

Typical runtime wiring:

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

## Storage

| Storage item | Key | Value | Description |
|---|---|---|---|
| `PaymentIntents` | `PaymentIntentId` | `PaymentIntent` | Full intent records |
| `IntentFundingAccounts` | `PaymentIntentId` | `AccountId` | Funding account for hold-settlement intents; present only while status is `Funded` |
| `PaymentIntentsByPayer` | `(IdentityId, PaymentIntentId)` | `()` | Sparse index for listing a payer's intents |
| `PaymentIntentsByPayee` | `(IdentityId, PaymentIntentId)` | `()` | Sparse index for listing a payee's intents |

### Hold reason

```rust
pub enum HoldReason {
    PaymentIntent,  // Native balance held while an intent is in `Funded` state
}
```

Holds are placed and released via the `fungible::hold::Mutate` interface to ensure
the held amount is reserved exclusively for this pallet and cannot be spent.

---

## State Machine

```
                  create_payment_intent
                          │
                          ▼
                      Requested
                     /    │    \
          fund(Direct)    │     cancel_payment_intent
               │          │              │
               ▼          │              ▼
            Claimed    fund(Hold)    Cancelled  ◄─ terminal
                          │
                          ▼
                        Funded
                       /      \
          claim_payment_intent  refund_payment_intent
                  │                      │
                  ▼                      ▼
               Claimed ◄─ terminal   Refunded ◄─ terminal

          (from Requested, after expires_at)
                  │
          expire_payment_intent
                  │
                  ▼
               Expired ◄─ terminal
```

Terminal states: `Claimed`, `Refunded`, `Cancelled`, `Expired`.

---

## Settlement Modes

### Direct (`SettlementMode::Direct`)

`fund_payment_intent` immediately calls `Currency::transfer` from the funding
account to the current `owner_account` of the payee identity. The intent moves
directly to `Claimed` in the same extrinsic. No hold is placed; no `claim` step
is required.

```
fund(Direct)
  └─ transfer(funding_account → payee_owner, amount)
  └─ status: Funded → Claimed
```

### Hold (`SettlementMode::Hold`)

`fund_payment_intent` places a hold on `amount` in the funding account via
`Currency::hold(HoldReason::PaymentIntent)`. The intent moves to `Funded` and the
funding account is recorded in `IntentFundingAccounts`.

```
fund(Hold)
  └─ hold(funding_account, amount)
  └─ status: Requested → Funded

claim_payment_intent
  └─ transfer_on_hold(funding_account → payee_owner, amount)
  └─ status: Funded → Claimed

refund_payment_intent
  └─ release(funding_account, amount)
  └─ status: Funded → Refunded
```

---

## Dispatchable Calls

| Index | Call | Authority | Description |
|---|---|---|---|
| 0 | `create_payment_intent(intent_id, payer, payee, asset_id, amount, action, memo_ref, settlement_mode, expires_at)` | `CAP_MANAGE_PAYMENT` for payer | Create a `Requested` intent; `asset_id` must be `0`; `amount` must be non-zero |
| 1 | `fund_payment_intent(intent_id)` | `CAP_MANAGE_PAYMENT` for payer | Fund according to settlement mode; intent must be `Requested` and not expired |
| 2 | `claim_payment_intent(intent_id, evidence_ref)` | `CAP_MANAGE_PAYMENT` for payee | Transfer held funds to payee owner; intent must be `Funded` |
| 3 | `refund_payment_intent(intent_id, evidence_ref)` | `CAP_MANAGE_PAYMENT` for payer | Release held funds back to funding account; intent must be `Funded` |
| 4 | `cancel_payment_intent(intent_id)` | `CAP_MANAGE_PAYMENT` for payer | Cancel before any funds move; intent must be `Requested` |
| 5 | `expire_payment_intent(intent_id)` | Any signed | Mark `Requested` intent as `Expired` once `expires_at` has passed |

### Authority notes

- Authority is checked via `T::IdentityProvider::ensure_can_manage_payment` for all
  payer-side calls, and `ensure_can_claim_payment` for claim.
- `expire_payment_intent` requires only a valid signed origin because the timestamp
  check and state transition are fully deterministic.
- `evidence_ref` parameters on `claim` and `refund` are reserved for future
  off-chain evidence anchoring and are currently ignored.

---

## Events

```rust
pub enum Event<T: Config> {
    PaymentIntentCreated {
        intent_id,
        payer: IdentityId,
        payee: IdentityId,
        asset_id: AssetId,
        amount: Amount,
        action: PaymentAction,
    },
    PaymentIntentFunded {
        intent_id,
        settlement_mode: SettlementMode,
    },
    PaymentIntentClaimed   { intent_id },
    PaymentIntentRefunded  { intent_id },
    PaymentIntentCancelled { intent_id },
    PaymentIntentExpired   { intent_id },
}
```

---

## Errors

| Error | Meaning |
|---|---|
| `IntentAlreadyExists` | The supplied `intent_id` is already in use |
| `IntentNotFound` | No intent for the given `intent_id` |
| `InvalidState` | Current status does not permit the requested transition |
| `Unauthorized` | Caller not authorized by the identity's capability model |
| `InvalidAmount` | Amount is zero |
| `InvalidAsset` | `asset_id` is not `0` |
| `InvalidAction` | Action namespace is empty or otherwise invalid |
| `InvalidSettlementMode` | Settlement mode is unsupported |
| `FundingUnavailable` | Hold-settlement funding account record is missing |
| `InsufficientBalance` | Funding account cannot cover the hold or transfer |
| `AlreadyExpired` | Intent has expired; cannot fund |
| `NotYetExpired` | `expires_at` has not been reached; cannot expire |
| `ClaimNotAllowed` | Claim transition not permitted in current state |
| `RefundNotAllowed` | Refund transition not permitted in current state |
| `CancelNotAllowed` | Cancel not allowed (intent is not `Requested`) |
| `ExpireNotAllowed` | Expire not allowed (intent is not `Requested`) |
| `EvidenceInvalid` | Evidence content reference is invalid |
| `NonceInvalid` | Nonce mismatch |
| `Overflow` | Arithmetic overflow |
| `InvalidInput` | Generic malformed input |

---

## Testing

```bash
cargo test -p pallet-payment-intent
```

Key test scenarios covered by `src/tests.rs`:

- `create_and_direct_fund_works` — create intent, fund with Direct mode, assert `Claimed`
- `hold_claim_and_refund_state_machine_works` — full Hold cycle: create → fund → claim; and create → fund → refund
- Unauthorized caller rejection for each call
- Expiry enforcement: expire after `expires_at`, reject before
- Cancel before funding
- Error cases: duplicate `intent_id`, zero amount, wrong `asset_id`

---

## Benchmarks

`src/benchmarking.rs` provides FRAME benchmarks for all dispatchable calls:

```bash
cargo build --release --features runtime-benchmarks
./target/release/vibly-node benchmark pallet \
  --pallet pallet_payment_intent \
  --extrinsic "*" \
  --steps 50 --repeat 20 \
  --output pallets/payment-intent/src/weights.rs
```
