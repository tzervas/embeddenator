# Embeddenator Dependency Graph

## Visual Dependency Map

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         EMBEDDENATOR ECOSYSTEM                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌────────────────────┐        ┌─────────────────────┐                    │
│   │    embeddenator    │◄───────┤  embeddenator-cli   │                    │
│   │      (main)        │        │   (user interface)  │                    │
│   │     v0.19.4        │        │       v0.1.0        │                    │
│   └─────────┬──────────┘        └──────────┬──────────┘                    │
│             │                              │                               │
│             │ contains all                 │ depends on (git):             │
│             │ implementations              ├─► embeddenator-vsa            │
│             │ being extracted              ├─► embeddenator-retrieval      │
│             ▼                              ├─► embeddenator-fs             │
│                                            └─► embeddenator-io             │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                      CORE COMPONENT LAYER                            │  │
│   │                                                                      │  │
│   │   ┌──────────────────────┐                                          │  │
│   │   │   embeddenator-vsa   │  ◄─── Foundational (no internal deps)    │  │
│   │   │      v0.1.0          │                                          │  │
│   │   │  • HolographicVector │                                          │  │
│   │   │  • bind/unbind       │                                          │  │
│   │   │  • CodeBook          │                                          │  │
│   │   └──────────┬───────────┘                                          │  │
│   │              │                                                       │  │
│   │              │ depends on                                            │  │
│   │              ▼                                                       │  │
│   │   ┌──────────────────────────┐                                      │  │
│   │   │  embeddenator-retrieval  │  ◄─── Uses VSA for resonator         │  │
│   │   │        v0.2.0            │                                      │  │
│   │   │  • ResonatorNetwork      │                                      │  │
│   │   │  • SignatureStore        │                                      │  │
│   │   │  • HierarchicalRetrieval │                                      │  │
│   │   └──────────┬───────────────┘                                      │  │
│   │              │                                                       │  │
│   │              │ depends on                                            │  │
│   │              ▼                                                       │  │
│   │   ┌──────────────────────────┐                                      │  │
│   │   │    embeddenator-fs       │  ◄─── Uses VSA + Retrieval           │  │
│   │   │        v0.2.0            │                                      │  │
│   │   │  • EmbrFS                │                                      │  │
│   │   │  • FuseShim              │                                      │  │
│   │   │  • File encoding         │                                      │  │
│   │   └──────────┬───────────────┘                                      │  │
│   │              │                                                       │  │
│   │              │ depends on                                            │  │
│   │              ▼                                                       │  │
│   │   ┌──────────────────────────┐                                      │  │
│   │   │  embeddenator-interop    │  ◄─── Uses VSA + FS                  │  │
│   │   │        v0.2.0            │                                      │  │
│   │   │  • Kernel hooks          │                                      │  │
│   │   │  • System bridges        │                                      │  │
│   │   └──────────────────────────┘                                      │  │
│   │                                                                      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    STANDALONE INFRASTRUCTURE                         │  │
│   │                                                                      │  │
│   │   ┌──────────────────────┐    ┌──────────────────────┐             │  │
│   │   │   embeddenator-obs   │    │   embeddenator-io    │             │  │
│   │   │      v0.1.1          │    │      v0.1.1          │             │  │
│   │   │  • HighResTimer      │    │  • Envelope format   │             │  │
│   │   │  • MetricsCollector  │    │  • Serialization     │             │  │
│   │   │  • Logging macros    │    │                      │             │  │
│   │   └──────────────────────┘    └──────────────────────┘             │  │
│   │           │                            │                            │  │
│   │           └─── No internal deps ───────┘                            │  │
│   │                                                                      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    DEV/TEST COMPONENTS                               │  │
│   │                                                                      │  │
│   │   ┌─────────────────────────┐   ┌──────────────────────┐           │  │
│   │   │ embeddenator-contract-  │   │ embeddenator-testkit │           │  │
│   │   │        bench            │   │      v0.1.1          │           │  │
│   │   │       v0.2.1            │   │  • smoke_test()      │           │  │
│   │   │  • Golden hashes        │   │  (minimal - 3 LOC)   │           │  │
│   │   │  • Determinism          │   └──────────┬───────────┘           │  │
│   │   └──────────┬──────────────┘              │                        │  │
│   │              │                              │                        │  │
│   │              │ depends on                   │ depends on             │  │
│   │              ▼                              ▼                        │  │
│   │        embeddenator (path)           embeddenator (git v0.20.0)     │  │
│   │        embeddenator-io (opt)                                        │  │
│   │                                                                      │  │
│   │   ┌───────────────────────────┐                                     │  │
│   │   │  embeddenator-workspace   │  ◄─── Standalone utility            │  │
│   │   │    v0.1.0-alpha.1         │                                     │  │
│   │   │  (minimal - 63 LOC)       │                                     │  │
│   │   └───────────────────────────┘                                     │  │
│   │                                                                      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Dependency Matrix

### Internal Dependencies (embeddenator-* crates)

| Component | vsa | retrieval | fs | interop | obs | io |
|-----------|:---:|:---------:|:--:|:-------:|:---:|:--:|
| **embeddenator-vsa** | - | ❌ | ❌ | ❌ | ❌ | ❌ |
| **embeddenator-retrieval** | ✅ | - | ❌ | ❌ | ❌ | ❌ |
| **embeddenator-fs** | ✅ | ✅ | - | ❌ | ❌ | ❌ |
| **embeddenator-interop** | ✅ | ❌ | ✅ | - | ❌ | ❌ |
| **embeddenator-cli** | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ |
| **embeddenator-obs** | ❌ | ❌ | ❌ | ❌ | - | ❌ |
| **embeddenator-io** | ❌ | ❌ | ❌ | ❌ | ❌ | - |
| **embeddenator-contract-bench** | ❌ | ❌ | ❌ | ❌ | ❌ | ⚪ |
| **embeddenator-testkit** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **embeddenator-workspace** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

*✅ = Required dependency, ⚪ = Optional dependency, ❌ = No dependency*

---

## Build Order

The following build order respects the dependency graph (leaves first):

```
Stage 1 (No internal deps - can build in parallel):
├── embeddenator-vsa
├── embeddenator-obs
├── embeddenator-io
└── embeddenator-workspace

Stage 2 (Depends on Stage 1):
└── embeddenator-retrieval  (needs: vsa)

Stage 3 (Depends on Stage 2):
└── embeddenator-fs  (needs: vsa, retrieval)

Stage 4 (Depends on Stage 3):
└── embeddenator-interop  (needs: vsa, fs)

Stage 5 (Depends on Stage 3):
└── embeddenator-cli  (needs: vsa, retrieval, fs, io)

Stage 6 (Depends on main embeddenator):
├── embeddenator-contract-bench
└── embeddenator-testkit
```

---

## Dependency Resolution Strategy

### Current State

| Component | Resolution Method | Issues |
|-----------|-------------------|--------|
| embeddenator-vsa | path `"../embeddenator-vsa"` | Local dev only |
| embeddenator-retrieval | path | Local dev only |
| embeddenator-fs | path | Local dev only |
| embeddenator-interop | path | Local dev only |
| embeddenator-cli | **git tags** | Breaks local dev |
| embeddenator-testkit | git tag `v0.20.0` | Version mismatch |

### Recommended Strategy

For **development**:
```toml
[dependencies]
embeddenator-vsa = { path = "../embeddenator-vsa" }
```

For **release** (when published):
```toml
[dependencies]
embeddenator-vsa = "0.2"
```

### Workspace Cargo.toml (Future)

Once consolidated, use a Cargo workspace at the meta-level:

```toml
[workspace]
members = [
    "embeddenator-vsa",
    "embeddenator-retrieval",
    "embeddenator-fs",
    "embeddenator-interop",
    "embeddenator-obs",
    "embeddenator-io",
    "embeddenator-cli",
]

[workspace.dependencies]
embeddenator-vsa = { path = "embeddenator-vsa" }
embeddenator-retrieval = { path = "embeddenator-retrieval" }
# etc.
```

---

## External Dependencies (Key)

| Crate | Used By | Purpose |
|-------|---------|---------|
| `rand` | vsa, testkit | Random vector generation |
| `serde` | vsa, retrieval, fs, io | Serialization |
| `bincode` | vsa, retrieval, cli | Binary encoding |
| `sha2` | vsa, retrieval | Content addressing |
| `fuser` | fs | FUSE bindings |
| `clap` | cli, contract-bench | CLI parsing |
| `walkdir` | fs, cli | Directory traversal |
| `arc-swap` | fs | Lock-free pointer swap |
| `tracing` | obs | Structured logging |
| `rayon` | contract-bench | Parallelism |
| `tokio` | (various) | Async runtime |

---

## Code Duplication Alert

The `SignatureStore` type exists in **3 locations**:

1. `embeddenator/src/retrieval/signature.rs` ← Original
2. `embeddenator-retrieval/src/signature.rs` ← Extracted (disabled)
3. `embeddenator-fs/src/embr/signature.rs` ← Copy

**Action Required**: Consolidate to `embeddenator-retrieval` only. Update `embeddenator-fs` to import from retrieval.
