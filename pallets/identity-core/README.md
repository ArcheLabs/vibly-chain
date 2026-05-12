# pallet-identity-core

> 中文文档：[README.zh.md](README.zh.md)

A FRAME pallet that stores and governs root identities on the vibly-chain. It is
the single source of truth for owner accounts, optional recovery accounts, delegated
capability keys, active content pointers, and external transport bindings. All other
pallets that need identity information consume the `IdentityAccess` trait rather than
reading this pallet's storage directly.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Configuration](#configuration)
4. [Storage](#storage)
5. [Identity Lifecycle](#identity-lifecycle)
6. [Capability Model](#capability-model)
7. [Dispatchable Calls](#dispatchable-calls)
8. [Events](#events)
9. [Errors](#errors)
10. [Integration (IdentityAccess trait)](#integration-identityaccess-trait)
11. [Testing](#testing)
12. [Benchmarks](#benchmarks)

---

## Overview

Each **root identity** is identified by a deterministic `IdentityId` (a `H256` hash
derived from a monotonic pallet sequence). An identity carries:

| Field | Description |
|---|---|
| `owner` | The primary `AccountId`; has unrestricted authority over the identity |
| `recovery` | Optional `AccountId` that can perform owner/recovery lifecycle actions |
| `active_profile` | Optional `ContentRef` pointing to the off-chain profile document |
| `active_agent_registry` | Optional `ContentRef` for the agent registry document |
| `active_auth_registry` | Optional `ContentRef` for the authorization registry |
| `active_relation_policy` | Optional `ContentRef` for the relation policy document |
| `status` | `Active` \| `Frozen` \| `Disabled` |
| `nonce` | Monotonic mutation counter; bumped on every state change |
| `created_at` / `updated_at` | Millisecond timestamps from `TimeProvider` |

---

## Architecture

```
             ┌──────────────────────────────────────────────────────┐
             │                  pallet-identity-core                 │
             │                                                        │
             │  Identities  AuthorizedKeys  TransportBindings        │
             │       │             │               │                  │
             │       └─────────────┴───────────────┘                 │
             │                    ▲                                   │
             │          IdentityAccess trait                         │
             └──────────────────────────────────────────────────────┘
                        ▲                    ▲
               pallet-payment-intent    (any future pallet)
```

Other pallets call `T::IdentityProvider::ensure_can_manage_payment(identity, account)?`
(or similar methods) instead of querying `Identities` storage directly. This keeps the
identity model as a single pluggable dependency.

---

## Configuration

```rust
pub trait Config: frame_system::Config {
    /// Weight provider for dispatchable calls.
    type WeightInfo: WeightInfo;

    /// Timestamp provider returning milliseconds since epoch.
    type TimeProvider: Time<Moment = u64>;

    /// Maximum byte length of a content CID.
    #[pallet::constant]
    type MaxCidLen: Get<u32>;

    /// Maximum byte length of a content URI.
    #[pallet::constant]
    type MaxUriLen: Get<u32>;

    /// Maximum byte length of an external transport account locator.
    #[pallet::constant]
    type MaxTransportAccountLen: Get<u32>;
}
```

Typical runtime wiring:

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

## Storage

| Storage item | Key | Value | Description |
|---|---|---|---|
| `Identities` | `IdentityId` | `RootIdentity` | Root identity records |
| `AuthorizedKeys` | `KeyId` | `AuthorizedKeyRecord` | Delegated key records |
| `AuthorizedKeyIdByAccount` | `(IdentityId, AccountId)` | `KeyId` | Reverse lookup from account to key ID |
| `TransportBindings` | `TransportBindingId` | `TransportBinding` | External transport binding records |
| `TransportBindingByIdentityAndLocator` | `Hash256` of `(identity, transport, account)` | `TransportBindingId` | Uniqueness guard for transport locators |
| `NextIdentitySequence` | — | `u64` | Monotonic counter for identity ID derivation |
| `NextTransportSequence` | — | `u64` | Monotonic counter for transport binding ID derivation |

### ID Derivation

Identity IDs and transport binding IDs are **not** sequential integers; they are
`BlakeTwo256` hashes of a domain-separation prefix plus the current counter value:

```
IdentityId        = BlakeTwo256( ("vibly/identity",  seq) )
TransportBindingId = BlakeTwo256( ("vibly/transport", seq) )
KeyId             = BlakeTwo256( ("vibly/key", identity_id, account) )
```

This makes IDs unpredictable and avoids enumeration attacks.

---

## Identity Lifecycle

```
                register_identity
                       │
                       ▼
                    Active
                    ┌────┐
                    │    │  ← delegated ops (pointers, transports, keys)
                    │    │  ← owner / recovery ops
                    └────┘
              freeze_identity│
                    ▼
                  Frozen          ← only owner/recovery ops allowed
              unfreeze_identity│
                    ▲
                    │
              disable_identity (from Active or Frozen)
                    ▼
                 Disabled         ← immutable, no ops allowed
```

- **Frozen**: Delegated pointer, transport, and payment-management actions are rejected.
  Owner and recovery can still rotate the owner key, set/clear recovery, unfreeze, or
  disable the identity.
- **Disabled**: The identity record is preserved for auditability but no further
  mutations are accepted.

---

## Capability Model

Delegated keys carry a `CapabilityMask` bitmask. The pallet enforces four built-in
access scopes:

| Constant | Bit | Permitted operations |
|---|---|---|
| `CAP_ADMIN` | `0x01` | Owner/recovery lifecycle (rotate owner, set recovery, freeze, unfreeze, disable) |
| `CAP_MANAGE_POINTERS` | `0x02` | Content pointer mutations; also required to add/revoke keys |
| `CAP_MANAGE_TRANSPORTS` | `0x04` | Create and revoke transport bindings |
| `CAP_MANAGE_PAYMENT` | `0x08` | Cross-pallet payment authorization (`IdentityAccess::ensure_can_manage_payment`) |

The **owner** account always has full authority over all scopes. The **recovery**
account is limited to `OwnerOrRecovery`-scoped operations only (lifecycle management).
Delegated keys can hold any combination of the four bits.

### Key record fields

```rust
pub struct AuthorizedKeyRecord<AccountId> {
    pub key_id:          KeyId,
    pub identity_id:     IdentityId,
    pub account:         AccountId,
    pub purpose:         KeyPurpose,    // e.g. Finance, Signing, …
    pub capability_mask: CapabilityMask,
    pub expires_at:      Option<u64>,   // milliseconds; None = no expiry
    pub revoked_at:      Option<u64>,
    pub created_at:      u64,
}
```

---

## Dispatchable Calls

### Identity lifecycle

| Index | Call | Minimum authority | Description |
|---|---|---|---|
| 0 | `register_identity(recovery, active_profile, active_agent_registry, active_auth_registry, active_relation_policy)` | Any signed | Create a new root identity owned by the signer |
| 1 | `rotate_owner_key(identity_id, new_owner)` | Owner **or** Recovery | Replace the owner account |
| 2 | `set_recovery_key(identity_id, new_recovery)` | Owner **or** Recovery | Set or clear the recovery account |
| 12 | `freeze_identity(identity_id)` | Owner **or** Recovery | Suspend delegated operations |
| 13 | `unfreeze_identity(identity_id)` | Owner **or** Recovery | Reactivate a frozen identity |
| 14 | `disable_identity(identity_id)` | Owner **or** Recovery | Permanently disable (irreversible) |

### Delegated key management

| Index | Call | Minimum authority | Description |
|---|---|---|---|
| 3 | `add_key(identity_id, account, purpose, capability_mask, expires_at)` | Owner **or** `CAP_MANAGE_POINTERS` | Add a delegated key; `Owner`/`Recovery` purposes are rejected |
| 4 | `revoke_key(identity_id, key_id)` | Owner **or** `CAP_MANAGE_POINTERS` | Remove a delegated key and its reverse-lookup entry |

### Content pointers

All pointer calls share the same authority: owner or any delegated key with
`CAP_MANAGE_POINTERS`. The identity must be `Active`.

| Index | Call | Description |
|---|---|---|
| 5 | `set_active_profile(identity_id, profile)` | Set or clear the profile pointer |
| 6 | `set_active_agent_registry(identity_id, registry)` | Set or clear the agent registry pointer |
| 7 | `set_active_auth_registry(identity_id, registry)` | Set or clear the auth registry pointer |
| 8 | `set_active_relation_policy(identity_id, policy)` | Set or clear the relation policy pointer |

### Transport bindings

| Index | Call | Minimum authority | Description |
|---|---|---|---|
| 9 | `bind_transport(identity_id, transport, account, proof_ref)` | Owner **or** `CAP_MANAGE_TRANSPORTS` | Create a `Pending` transport binding; locator must be unique |
| 10 | `verify_transport(identity_id, binding_id, proof_ref)` | Owner **or** Recovery only | Promote a binding to `Verified`; asserts identity control |
| 11 | `revoke_transport(identity_id, binding_id)` | Owner **or** `CAP_MANAGE_TRANSPORTS` | Mark a binding `Revoked` (record kept for auditability) |

> **Note:** `verify_transport` requires the owner or recovery account specifically,
> because verification asserts control over the identity itself rather than delegated
> transport-management authority.

---

## Events

```rust
pub enum Event<T: Config> {
    IdentityRegistered       { identity_id, owner },
    OwnerKeyRotated          { identity_id, old_owner, new_owner },
    RecoveryKeySet           { identity_id },
    IdentityKeyAdded         { identity_id, key_id, purpose },
    IdentityKeyRevoked       { identity_id, key_id },
    ActiveProfileSet         { identity_id },
    ActiveAgentRegistrySet   { identity_id },
    ActiveAuthRegistrySet    { identity_id },
    ActiveRelationPolicySet  { identity_id },
    TransportBound           { identity_id, binding_id, transport },
    TransportVerified        { identity_id, binding_id },
    TransportRevoked         { identity_id, binding_id },
    IdentityFrozen           { identity_id },
    IdentityUnfrozen         { identity_id },
    IdentityDisabled         { identity_id },
}
```

---

## Errors

| Error | Meaning |
|---|---|
| `IdentityAlreadyExists` | The generated identity ID collided (should not happen in practice) |
| `IdentityNotFound` | No record for the given `IdentityId` |
| `InvalidState` | Operation not allowed in the current identity or binding state |
| `AlreadyFrozen` / `NotFrozen` | Freeze/unfreeze guard |
| `AlreadyDisabled` | Identity already permanently disabled |
| `Unauthorized` | Caller lacks owner, recovery, or required capability bit |
| `OwnerKeyRequired` | Call requires the owner key specifically |
| `RecoveryNotConfigured` | No recovery account is set |
| `RecoveryNotAllowed` | Recovery account cannot perform this particular operation |
| `KeyAlreadyExists` | A delegated key for this account already exists |
| `KeyNotFound` | No delegated key for the given `KeyId` |
| `KeyInvalid` | Key would duplicate owner/recovery, or purpose is `Owner`/`Recovery` |
| `KeyExpired` | Key's `expires_at` is in the past |
| `KeyRevoked` | Key has been revoked |
| `PointerInvalid` | Content reference is structurally invalid |
| `TransportBindingAlreadyExists` | Locator `(identity, transport, account)` is already bound |
| `TransportBindingNotFound` | No binding for the given `TransportBindingId` |
| `TransportVerificationFailed` | Proof validation failed |
| `TransportNotAllowed` | Transport kind or account is not permitted |
| `NonceInvalid` | Supplied nonce does not match |
| `Overflow` | Sequence counter overflow (theoretical) |
| `InvalidInput` | Generic malformed input |

---

## Integration (IdentityAccess trait)

Other pallets declare a `type IdentityProvider: IdentityAccess<Self::AccountId>`
configuration item and call:

```rust
// Verify the identity exists.
T::IdentityProvider::identity_exists(&identity_id);

// Require the caller has payment-management authority for the identity.
T::IdentityProvider::ensure_can_manage_payment(&identity_id, &who)?;

// Require the caller has payment-claim authority for the identity (payee side).
T::IdentityProvider::ensure_can_claim_payment(&identity_id, &who)?;

// Resolve the owner account for fund transfers.
let owner = T::IdentityProvider::owner_account(&identity_id)?;
```

Wire the implementation in the runtime:

```rust
impl pallet_payment_intent::Config for Runtime {
    type IdentityProvider = IdentityCore;
    // …
}
```

---

## Testing

```bash
cargo test -p pallet-identity-core
```

Key test scenarios covered by `src/tests.rs`:

- `register_and_manage_identity_works` — full registration, pointer mutation, key addition
- `transport_flow_works` — bind → verify → revoke transport binding
- Owner rotation and recovery key management
- Freeze / unfreeze / disable state transitions
- Delegation authorization with capability mask enforcement
- Expiry and revocation of delegated keys

---

## Benchmarks

`src/benchmarking.rs` provides FRAME benchmarks for all dispatchable calls. Build
with the `runtime-benchmarks` feature and run against a benchmark-enabled node:

```bash
cargo build --release --features runtime-benchmarks
./target/release/vibly-node benchmark pallet \
  --pallet pallet_identity_core \
  --extrinsic "*" \
  --steps 50 --repeat 20 \
  --output pallets/identity-core/src/weights.rs
```
