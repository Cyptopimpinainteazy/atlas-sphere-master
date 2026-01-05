# X3 Constitutional Specification — Release Package v1.0

**Release Date:** 2025-01-16
**Status:** IMMUTABLE
**Spec Version:** X3-CONSTITUTIONAL-V1

---

## Package Contents

### Core Specification

| File | Description | SHA-256 |
|------|-------------|---------|
| `x3-inv-c.md` | Canonical constitutional specification | `41e855e50c2f07899c69f30ef963bc6c820f01b68bdf1d113472b08846589cbb` |
| `x3-inv-c.map.md` | Implementation mapping (spec→code) | `516055ddd5e65d5eaedaf2fef23970117a84c582582deff909cdf149b5182e0b` |

### Legal & Positioning

| File | Description | SHA-256 |
|------|-------------|---------|
| `LEGAL_PREAMBLE.md` | Formal legal & interpretive preamble | `5e7358a4aa21435255ee5e2e5d0b567c9f4e67731d575174c1e9e20d81f12d0b` |
| `THREAT_MODEL.md` | Auditor-ready threat model | `cf3e2635b1a0c31057a30e6c32cd2e25e987596b21ce5e91d30c1386db728ba8` |
| `WHAT_X3_IS_NOT.md` | Adversarial manifesto | `f013b3f16049d8c0321f17bcdeb59b90a8bc2d05b9ca2451c49566a8384c73af` |

### Formal Verification

| File | Description | SHA-256 |
|------|-------------|---------|
| `X3FailureEscalation.tla` | TLA+ counterexample search (failure paths) | `462c33cb6d5f6b652ef766c1911d0df4b023216956b9ad1d4445b6476dceda3e` |
| `X3EconomicSecurity.tla` | TLA+ counterexample search (economic attacks) | `0e70c0a56c4a4474bb8ee699e4ab1ea316219a4afbc42844b629e455bd368c42` |
| `x3-inv.k` | K Framework semantic definitions | `59088526c83ad561254b08cf5bccda284439219fe96f7d5632067b7e5963e943` |
| `test-vectors.json` | Canonical test vectors (32 cases) | `c103a462eb790d9abe213020eee014f872c0012865e301a8874fe56560d0d28e` |

---

## Package Hash

**Merkle Root of Content Files (excludes README):**
```
afe5ca6d881802952a8b1ed626e4c4662179de102d2a9dd5b7a065971e8c4dce
```

**Verification:**
```bash
cd spec/x3-spec-v1
sha256sum LEGAL_PREAMBLE.md THREAT_MODEL.md WHAT_X3_IS_NOT.md \
  X3EconomicSecurity.tla X3FailureEscalation.tla x3-inv-c.map.md \
  x3-inv-c.md x3-inv.k test-vectors.json | sha256sum
```

---

## How to Use This Package

### For Auditors

1. **Start with:** `x3-inv-c.md` — Read the canonical constitutional invariants
2. **Verify implementations:** `x3-inv-c.map.md` — Trace every clause to Rust code
3. **Hunt for attacks:** Run TLA+ files in TLC model checker
4. **Understand philosophy:** `WHAT_X3_IS_NOT.md`

### For Developers

1. **Reference:** `crates/x3-inv-compiler/src/constitutional.rs` implements all invariants
2. **Test vectors:** `test-vectors.json` contains 32 canonical test cases
3. **K Framework:** `x3-inv.k` for formal proofs

### For Model Checking

```bash
# Install TLA+ Toolbox or use command-line TLC

# Counterexample search - failure paths
tlc X3FailureEscalation.tla -config X3FailureEscalation.cfg -deadlock

# Counterexample search - economic attacks
tlc X3EconomicSecurity.tla -config X3EconomicSecurity.cfg -deadlock
```

**Configuration files** (create these):

`X3FailureEscalation.cfg`:
```
CONSTANTS
  Domains = {"EVM", "SVM", "BTC", "X3"}
  Guardians = {"G1", "G2", "G3", "G4", "G5"}
  MaxBlocks = 20
  MinStake = 1000
  SlashPercent = 100
  ChallengeWindow = 5
  GracePeriod = 2

INVARIANT HuntForgedReceiptSuccess
INVARIANT HuntQuorumBypass
INVARIANT HuntSilentTimeout
INVARIANT HuntFreeOption
INVARIANT HuntGracePeriodAbuse
INVARIANT HuntSplitBrain
```

`X3EconomicSecurity.cfg`:
```
CONSTANTS
  Actors = {"A1", "A2", "A3", "A4", "A5"}
  SwapValue = 10000
  MinStakeRatio = 10
  DetectionProb = 90
  CollusionThreshold = 3
  MaxRounds = 10

INVARIANT HuntProfitableAttack
INVARIANT HuntCollusionProfit
INVARIANT HuntGriefing
INVARIANT HuntLowSlash
```

---

## Verification Commands

### Run All Tests

```bash
cd /media/lojak/sda1/atlas-sphere-master

# Constitutional invariant tests
cargo test -p x3-inv-compiler --lib constitutional

# All closure mechanics tests
cargo test -p x3-liveness -p x3-dispute -p x3-economics -p x3-finality -p x3-audit-trail
```

### Verify File Integrity

```bash
cd spec/x3-spec-v1

# Check individual file
sha256sum x3-inv-c.md
# Expected: 5af0d9a54ddfd24070aa29ca5116f025e632393ac3393351e07a4064aca77e80

# Check all files
sha256sum -c <<EOF
5af0d9a54ddfd24070aa29ca5116f025e632393ac3393351e07a4064aca77e80  x3-inv-c.md
f50bcd681a133b8bf6df258ddb491202afd9719936ca7cd40c80bc7abc052492  x3-inv-c.map.md
f013b3f16049d8c0321f17bcdeb59b90a8bc2d05b9ca2451c49566a8384c73af  WHAT_X3_IS_NOT.md
462c33cb6d5f6b652ef766c1911d0df4b023216956b9ad1d4445b6476dceda3e  X3FailureEscalation.tla
0e70c0a56c4a4474bb8ee699e4ab1ea316219a4afbc42844b629e455bd368c42  X3EconomicSecurity.tla
59088526c83ad561254b08cf5bccda284439219fe96f7d5632067b7e5963e943  x3-inv.k
c103a462eb790d9abe213020eee014f872c0012865e301a8874fe56560d0d28e  test-vectors.json
EOF
```

---

## Constitutional Hash

The Rust implementation computes a deterministic constitutional hash:

```rust
use x3_inv_compiler::constitutional::constitutional_hash;

let hash = constitutional_hash();
// Returns SHA-256 of all constitutional invariant definitions
```

This hash anchors the immutable specification to the code.

---

## Amendment Policy

**This specification is immutable.**

Changes require:
1. New version number (v2.0, etc.)
2. Migration path from previous version
3. Full re-audit
4. New package with new hashes

**Governance cannot override constitutional invariants during execution.**

---

## Contact

- Repository: `atlas-sphere-master`
- Branch: `fix/workspace-ci`
- Specification maintained in: `spec/x3-spec-v1/`

---

*This package is adversary-hostile by design. Questions from auditors welcome. Complaints from attackers expected.*
