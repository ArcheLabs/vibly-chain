# Security Policy

vibly-chain is preparing for public testnet use. Treat the repository as
pre-mainnet software unless a release explicitly says otherwise.

## Reporting a Vulnerability

Do not open a public issue for a suspected vulnerability. Send a private report
to the maintainers through the GitHub security advisory flow for
`ArcheLabs/vibly-chain`, or contact the maintainers through the private channel
listed by the project team.

Include:

- Affected commit or release.
- Reproduction steps.
- Expected and actual behavior.
- Potential impact.
- Any suggested mitigation.

## Scope

Security-sensitive areas include:

- Identity owner, recovery, delegated-key, and capability checks.
- Payment intent funding, hold, claim, refund, cancel, and expiry transitions.
- Runtime configuration, genesis presets, and testnet artifact generation.
- Node RPC exposure and collator launch configuration.

## Response Expectations

Maintainers should acknowledge complete reports promptly, triage severity, and
coordinate disclosure timing before public details are posted.
