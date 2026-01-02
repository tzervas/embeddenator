# ADR-013: Hierarchical Manifest Format

## Status

**Accepted** (implemented in v0.3.0)

## Date

2026-01-01

## Context

The original Embeddenator architecture supported a flat engram format: a single VSA vector representing an entire input (file or directory) with minimal metadata. While effective for simple "blob" ingestion and retrieval, this approach lacked the structural information necessary for advanced workflows:

### Limitations of Flat Format

- **No Path Information**: Cannot selectively query or extract specific files by path (e.g., "retrieve `src/lib.rs` from this engram").
- **Opaque Structure**: Flat engrams are black boxes; users cannot inspect what files/directories were bundled without full decoding.
- **All-or-Nothing Retrieval**: Must reconstruct the entire engram to access any part of it; no selective unfolding.
- **Limited Metadata**: No hierarchy levels, directory relationships, or structural annotations.

### Two Distinct Use Cases

1. **Simple Blob Ingestion**: Single files or directories treated as atomic units (e.g., embedding a README, a log file, or a small config directory).
   - Flat format is optimal: minimal overhead, simple implementation.
2. **Structured Hierarchical Organization**: Large directory trees requiring selective access, partial extraction, or path-based queries (e.g., codebases, documentation repositories).
   - Needs structured metadata to support:
     - Query by path pattern (e.g., "all `*.rs` files")
     - Selective unfolding (e.g., "extract `src/` subtree without rebuilding `tests/`")
     - Inspection of directory structure without full decode

### Existing Specification Without Rationale

The `docs/HIERARCHICAL_FORMAT.md` specification document describes the hierarchical manifest structure in detail, but lacks architectural context:

- Why introduce this format instead of enhancing the flat format?
- What alternatives were considered?
- What trade-offs were made?
- How does versioning and evolution work?

This ADR provides the missing architectural rationale and decision context.

## Decision

We introduce a **dual-format manifest system** with both flat and hierarchical variants, unified under a single `UnifiedManifest` enum:

```rust
pub enum UnifiedManifest {
    Flat(FlatManifest),
    Hierarchical(HierarchicalManifest),
}
```

### Flat Manifest (Existing)

Retained for simple blob ingestion:
- Contains single `chunks: Vec<ChunkRef>` with byte ranges
- Optional basic metadata (name, hash, size)
- Minimal serialization overhead

### Hierarchical Manifest (New)

Introduces structured metadata for directory hierarchies:

```rust
pub struct HierarchicalManifest {
    pub hierarchical_version: u32,  // Format version (starts at 1)
    pub root_path: String,
    pub levels: Vec<ManifestLevel>, // One per hierarchy depth
    pub sub_engrams: BTreeMap<String, SubEngramRef>,
}

pub struct ManifestLevel {
    pub depth: usize,
    pub items: Vec<ManifestItem>,
}

pub struct ManifestItem {
    pub path: String,
    pub chunk_ids: Vec<usize>,
    pub item_type: ItemType, // File or Directory
}

pub struct SubEngramRef {
    pub path: String,
    pub artifact_file: String,  // Filename in artifacts directory
    pub hash: String,            // Content hash for integrity
}
```

### Key Design Properties

1. **Level-Based Organization**: `levels: Vec<ManifestLevel>` groups items by depth
   - Enables breadth-first traversal and depth-limited queries
   - Natural representation of filesystem hierarchy

2. **Path-Indexed Items**: Each `ManifestItem` includes full path
   - Direct lookup for path-based queries (e.g., `query-text "src/lib.rs"`)
   - Glob pattern matching support (future: `"src/**/*.rs"`)

3. **Separate Sub-Engram Tracking**: `sub_engrams: BTreeMap<String, SubEngramRef>`
   - References extracted directory components stored as separate artifacts
   - Enables lazy loading: load sub-engrams only when needed
   - Supports selective extraction without full engram decode

4. **JSON Serialization**: Human-readable and toolable
   - Easy inspection with `cat`, `jq`, `less`
   - Version control friendly (stable diffs via `BTreeMap` from ADR-009)
   - No Rust dependency for third-party tools

5. **Deterministic Serialization**: 
   - Uses `BTreeMap` for sorted key order (see ADR-009)
   - Ensures reproducible builds and stable git diffs

### CLI Command Differentiation

- **`ingest <input>`**: Creates flat manifest
  - Optimized for single files or small directories
  - Minimal metadata overhead
  
- **`bundle-hier <input>`**: Creates hierarchical manifest
  - Generates structured manifest + directory of sub-engram artifacts
  - Suitable for large codebases and structured datasets

### Format Versioning

- `hierarchical_version: 1` field enables evolution
- Semver-style compatibility:
  - **Minor version**: Backward-compatible additions (new optional fields)
  - **Major version**: Breaking changes (field removals, structural changes)
- Readers check version and handle accordingly (reject unsupported, migrate old versions)

## Alternatives Considered

### 1) Pure Graph Format (Nodes + Edges) — **Rejected**

**Idea**: Represent manifest as a graph with nodes (files/dirs) and edges (parent-child relationships).

```json
{
  "nodes": [
    {"id": "n1", "path": "src/", "type": "dir"},
    {"id": "n2", "path": "src/lib.rs", "type": "file"}
  ],
  "edges": [
    {"from": "n1", "to": "n2", "rel": "contains"}
  ]
}
```

**Rejection Reasons**:
- **Over-engineered**: Filesystems are trees, not arbitrary graphs; edges are implicit from paths.
- **Query Complexity**: Path-based queries require graph traversal instead of simple lookup.
- **Implementation Burden**: Requires graph algorithms for basic operations (find file, list directory).
- **Tooling Difficulty**: Harder to understand and manipulate with standard JSON tools.

### 2) Nested JSON (Tree Structure) — **Rejected**

**Idea**: Mirror filesystem structure directly in nested JSON.

```json
{
  "src": {
    "type": "dir",
    "children": {
      "lib.rs": {"type": "file", "chunks": [0, 1, 2]},
      "main.rs": {"type": "file", "chunks": [3, 4]}
    }
  }
}
```

**Rejection Reasons**:
- **Path Query Awkwardness**: Finding `src/lib.rs` requires recursive descent: `manifest["src"]["children"]["lib.rs"]`.
- **Deep Nesting**: Large directory trees create deeply nested JSON (hard to read, parse, diff).
- **Redundant Path Information**: Paths implicit in nesting; reconstruction requires tree traversal.
- **Inflexible**: Adding non-tree relationships (e.g., symlinks, cross-references) breaks structure.

### 3) Protocol Buffers / Binary Format — **Rejected**

**Idea**: Use efficient binary serialization (protobuf, bincode, etc.) instead of JSON.

**Rejection Reasons**:
- **Human Readability Lost**: Cannot inspect manifests with standard tools (`cat`, `less`, `jq`).
- **Tooling Dependency**: Third-party tools require Rust/protobuf library to parse manifests.
- **Version Control Unfriendly**: Binary diffs are meaningless; can't review manifest changes in git.
- **Debugging Difficulty**: Requires specialized tools to inspect; JSON is universally accessible.
- **Premature Optimization**: Manifest size is negligible compared to engram data; JSON compression (gzip) achieves similar density.

### 4) Single Format (No Flat Variant) — **Rejected**

**Idea**: Always use hierarchical manifest, even for single files.

**Rejection Reasons**:
- **Unnecessary Overhead**: Single files don't need `levels`, `sub_engrams`, directory structure.
- **Complexity for Simple Cases**: Users ingesting a single README shouldn't deal with hierarchical metadata.
- **API Confusion**: Forces all code paths to handle hierarchical structure, complicating simple blob operations.

### 5) Multiple Separate Files (Manifest + Index + Data) — **Rejected**

**Idea**: Split into separate files:
- `manifest.json` (structure)
- `index.dat` (chunk lookup table)
- `engram.bin` (VSA vectors)

**Rejection Reasons**:
- **Distribution Complexity**: Three files must stay synchronized; risk of missing/mismatched components.
- **Atomicity**: Manifest and index must be updated together; separate files complicate atomic updates.
- **Tooling Burden**: All tools must handle multi-file artifacts; single-file is simpler.
- **Compression Redundancy**: Tarball/zip would re-bundle anyway; no benefit to separation.

## Consequences

### Positive

✅ **Dual-Format Flexibility**: Supports both simple blob workflows (flat) and structured hierarchical workflows (hierarchical) with appropriate overhead for each.

✅ **Human-Readable Metadata**: JSON format enables:
- Inspection without specialized tools: `cat manifest.json | jq '.levels[0].items[] | select(.path | contains("src"))'`
- Version control with meaningful diffs
- Third-party integration without Rust dependency

✅ **Path-Based Queries**: Natural query model:
- `query-text --path "src/lib.rs"` (exact match)
- `query-text --path-pattern "src/**/*.rs"` (future: glob support)
- Efficient lookup via flat `items` array

✅ **Selective Extraction**: Sub-engram references enable:
- Extract `src/` subtree without loading `tests/`, `docs/`
- Lazy loading of hierarchy branches
- Partial decoding for large engrams

✅ **Extensible Format**: Version field enables evolution:
- Add new fields without breaking old readers (minor version bump)
- Migrate to v2 when needed (major version bump, conversion tool)
- Readers can support multiple versions gracefully

✅ **Deterministic Builds**: `BTreeMap` for sorted keys ensures:
- Reproducible manifests across runs (ADR-009)
- Stable git diffs
- CI/CD validation via hash comparison

✅ **Tooling Independence**: JSON format means:
- Python/Node.js/shell scripts can parse manifests
- No need to link against Rust library for inspection
- Standard tooling (`jq`, `yq`, etc.) works out-of-the-box

### Negative

⚠️ **Manifest Size Scales with Structure**: Large directory trees produce large manifests:
- 10,000 files → ~1-2 MB JSON (depending on path lengths)
- **Mitigation**: Compress manifests (gzip achieves 80-90% reduction for typical paths)
- **Mitigation**: Manifests are metadata, not data; size is manageable for most use cases
- **Future**: Add `ManifestLevel::summary` field with aggregate stats to avoid loading full manifest

⚠️ **Format Versioning Overhead**: Requires maintenance:
- Must handle version field in all readers/writers
- Conversion tools needed when introducing v2
- Deprecation policy for old versions
- **Mitigation**: Version 1 is stable; changes will be rare and backward-compatible when possible
- **Mitigation**: Follow semver principles; only major versions break compatibility

⚠️ **Dual API Surface**: Supporting both flat and hierarchical requires:
- Enum handling in all manifest-consuming code
- Potential confusion about which format to use when
- **Mitigation**: Clear CLI separation (`ingest` vs `bundle-hier`)
- **Mitigation**: Comprehensive documentation and examples
- **Mitigation**: Auto-detect manifest type in query/retrieve commands

### Neutral

**JSON vs YAML**: JSON chosen over YAML for:
- Simpler parser (no indentation sensitivity)
- Wider tooling support (`jq` ecosystem)
- Faster parsing (though not critical for manifests)
- Stricter format (less ambiguity, fewer edge cases)

**Level-Based vs Flat Item List**: Level grouping adds structure:
- Enables depth-limited queries
- Matches filesystem mental model
- Minor overhead (duplicate depth information)
- Alternative: single flat `Vec<ManifestItem>` with `depth` field per item (considered equivalent; level grouping chosen for semantic clarity)

## Format Versioning Strategy

### Version Field

All hierarchical manifests include:

```json
{
  "hierarchical_version": 1,
  ...
}
```

### Compatibility Rules

**Minor Version (1.x → 1.y where y > x)**:
- Backward compatible: adds new optional fields
- Old readers ignore unknown fields
- Example: Adding `created_at` timestamp field → version 1.1

**Major Version (1.x → 2.0)**:
- Breaking changes: removes fields, changes structure
- Readers must check version and reject/migrate
- Example: Changing `levels` to graph structure → version 2.0

### Migration Path

When introducing v2:

1. **Conversion Tool**: `embeddenator migrate-manifest v1-to-v2 <input>`
   - Reads v1 manifest
   - Converts to v2 structure
   - Writes new manifest + artifacts as needed

2. **Gradual Rollout**:
   - Writers produce v2 by default
   - Readers support both v1 and v2 during transition period
   - Deprecate v1 support after migration window (e.g., 1 year)

3. **Version Detection**:
   ```rust
   match manifest.hierarchical_version {
       1 => read_v1(manifest),
       2 => read_v2(manifest),
       v => Err(format!("Unsupported version {}", v)),
   }
   ```

### Future Extensions (Potential v1.x)

**v1.1: Timestamps** (backward-compatible)
```json
{
  "hierarchical_version": 1,
  "created_at": "2026-01-01T12:00:00Z",
  ...
}
```

**v1.2: Aggregate Statistics** (backward-compatible)
```json
{
  "levels": [
    {
      "depth": 0,
      "item_count": 42,
      "total_size": 1048576,
      "items": [...]
    }
  ]
}
```

**v2.0: Symlink Support** (breaking change)
```json
{
  "hierarchical_version": 2,
  "items": [
    {
      "path": "src/link",
      "type": "symlink",
      "target": "src/lib.rs"
    }
  ]
}
```

## Tooling Impact

### CLI Commands

**Flat Format**:
```bash
embeddenator ingest README.md --output readme.engram
# Creates: readme.engram (flat manifest + data)
```

**Hierarchical Format**:
```bash
embeddenator bundle-hier ./my-project --output my-project.engram
# Creates: 
#   my-project.engram (root manifest + top-level engram)
#   my-project.artifacts/ (directory of sub-engrams)
```

### Query Commands (Format-Agnostic)

```bash
embeddenator query-text "error handling" --engram my-project.engram
# Auto-detects manifest type (flat or hierarchical)
# For hierarchical: searches across all levels

embeddenator query-text "src/lib.rs" --path-filter "src/**" --engram my-project.engram
# Hierarchical: filters items by path before searching
# Flat: ignores path-filter (logs warning)
```

### Extract Commands

```bash
embeddenator extract my-project.engram --output ./restored/
# Flat: decodes entire engram, writes single output
# Hierarchical: reconstructs directory tree, selectively loads sub-engrams

embeddenator extract my-project.engram --path "src/" --output ./src-only/
# Hierarchical: loads only src/ sub-engram, skips others
# Flat: not supported (logs error)
```

### Third-Party Tools

**Inspect Manifest Structure** (no Rust required):
```bash
jq '.levels[] | .items[] | select(.item_type == "File") | .path' manifest.json
# Lists all files in hierarchical manifest

jq '.sub_engrams | keys' manifest.json
# Lists all extracted sub-engram names
```

**Validate Manifest** (Python example):
```python
import json

with open('manifest.json') as f:
    manifest = json.load(f)

if 'hierarchical_version' in manifest:
    # Hierarchical format
    print(f"Version: {manifest['hierarchical_version']}")
    print(f"Levels: {len(manifest['levels'])}")
    for level in manifest['levels']:
        print(f"  Depth {level['depth']}: {len(level['items'])} items")
else:
    # Flat format
    print(f"Chunks: {len(manifest['chunks'])}")
```

## Implementation References

### Code Locations

- **Manifest Types**: [src/embrfs.rs](src/embrfs.rs)
  - `UnifiedManifest` enum
  - `HierarchicalManifest` struct
  - `ManifestLevel`, `ManifestItem`, `SubEngramRef` types
  
- **Flat Format**: [src/embrfs.rs](src/embrfs.rs)
  - `FlatManifest` struct (pre-existing)
  
- **Hierarchical Bundling**: [src/embrfs.rs](src/embrfs.rs)
  - `bundle_hierarchically()` function
  - Deterministic sorting (ADR-009)
  
- **CLI Commands**:
  - `ingest`: [src/cli.rs](src/cli.rs) (flat format)
  - `bundle-hier`: [src/cli.rs](src/cli.rs) (hierarchical format)
  - `query-text`: [src/cli.rs](src/cli.rs) (format-agnostic)

### Tests

- **Format Detection**: `tests/hierarchical_artifacts_e2e.rs`
  - Validates `UnifiedManifest` enum dispatch
  
- **Hierarchical Structure**: `tests/hierarchical_unfolding.rs`
  - Validates level organization, path indexing
  
- **Determinism**: `tests/hierarchical_determinism.rs`
  - Ensures manifests are reproducible (ADR-009)
  
- **Sub-Engram Extraction**: `tests/hierarchical_artifacts_e2e.rs`
  - Tests selective loading of sub-engrams

### Documentation

- **Specification**: [docs/HIERARCHICAL_FORMAT.md](docs/HIERARCHICAL_FORMAT.md)
  - Detailed format specification
  - JSON schema examples
  - Field descriptions
  
- **Related ADRs**:
  - [ADR-009: Deterministic Hierarchical Artifacts](ADR-009-deterministic-hierarchical-artifacts.md) (sorted keys)
  - [ADR-010: Router+Shard Architecture](ADR-010-router-shard-bounded-indexing.md) (indexing implications)
  - [ADR-008: Bundling Semantics and Cost-Aware Hybrid](ADR-008-bundling-semantics-and-cost-aware-hybrid.md) (bundling math)

## Summary

The introduction of the hierarchical manifest format, unified with the existing flat format under `UnifiedManifest`, enables Embeddenator to support both simple blob ingestion and advanced structured hierarchical workflows. The JSON-based, deterministic manifest format provides human readability, tooling independence, and extensibility while maintaining minimal overhead for simple use cases. Format versioning ensures graceful evolution, and the dual-format design preserves simplicity where appropriate while enabling sophisticated path-based queries, selective extraction, and filesystem-like organization for complex inputs.

**Status**: Accepted and implemented in v0.3.0 (2026-01-01)
