# ADR-011: Multi-Input Namespace Management

## Status

**Accepted** (implemented in v0.3.0)

## Date

2026-01-01

## Context

Embeddenator's primary use case involves ingesting directory trees and creating hierarchical VSA engrams for semantic search and retrieval. Prior to v0.3.0, the CLI interface was limited to ingesting a **single directory** per invocation:

```bash
embeddenator ingest -i ./my_docs -o engram.bin
```

This constraint created friction for users working with multiple projects, disparate documentation sources, or selective directory subsets. Common workflows that were difficult or impossible included:

1. **Multi-Project Ingestion**: Users maintaining multiple codebases or documentation trees needed to run separate `ingest` commands and manage multiple engram files, then manually merge them.
2. **Selective Directory Ingestion**: Ingesting specific subdirectories from different locations (e.g., `./project_a/docs`, `./project_b/api`, `./shared/specs`) required workarounds like symlinking or copying files to a temporary staging directory.
3. **Mixed Source Integration**: Combining documentation from heterogeneous sources (internal docs, external references, vendor specifications) was cumbersome.

### The Path Collision Problem

The fundamental technical challenge with multi-input ingestion is **path collisions**. Consider two directory trees:

```
./project_a/
  docs/
    README.md
    architecture.md
  src/
    main.rs

./project_b/
  docs/
    README.md
    api_reference.md
  src/
    lib.rs
```

If both trees are ingested without disambiguation, the paths `docs/README.md` would collide, resulting in:

- **Silent Overwrites**: Later inputs overwrite earlier ones, causing data loss.
- **Ambiguous Retrieval**: Query results for "README" would be ambiguous—which `README.md` is the user referring to?
- **Unpredictable Behavior**: Users cannot reason about which version of a file is stored in the engram.

This violates Embeddenator's core principles of **deterministic, predictable behavior** and **data integrity**.

## Decision

We extend the CLI to accept **multiple input paths** via repeatable `-i/--input` arguments and introduce **automatic namespace prefixing** to prevent path collisions:

### 1) CLI API: Repeatable Input Arguments

The `ingest` command's `-i/--input` argument is changed from a single `PathBuf` to `Vec<PathBuf>`:

```rust
#[derive(Parser, Debug)]
pub struct IngestArgs {
    /// Input directory or directories to ingest (repeatable)
    #[arg(short = 'i', long = "input", required = true)]
    pub inputs: Vec<PathBuf>,
    
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
    
    // ... other args
}
```

**Usage Examples:**

```bash
# Single input (backward compatible)
embeddenator ingest -i ./my_docs -o engram.bin

# Multiple inputs (v0.3.0+)
embeddenator ingest -i ./project_a -i ./project_b -i ./shared -o engram.bin

# Mixed files and directories
embeddenator ingest -i ./docs -i ./specs/overview.md -i ./vendor_docs -o engram.bin
```

### 2) Automatic Namespace Prefixing

To prevent path collisions, each input directory is assigned a **namespace prefix** based on its **directory basename**:

- **Input**: `./project_a` → **Prefix**: `project_a`
- **Input**: `./some/path/to/docs` → **Prefix**: `docs`
- **Input**: `/home/user/specs` → **Prefix**: `specs`

All files and subdirectories within that input are prefixed:

```
./project_a/docs/README.md  → stored as: project_a/docs/README.md
./project_b/docs/README.md  → stored as: project_b/docs/README.md
```

This ensures that even if multiple inputs contain identically named paths, they remain distinct in the engram.

### 3) Collision Avoidance: Numeric Suffixes

If multiple inputs share the same basename, **numeric suffixes** are automatically appended:

```bash
embeddenator ingest -i ./docs -i ./other/docs -i ./vendor/docs -o engram.bin
```

Results in prefixes:
- `./docs` → `docs`
- `./other/docs` → `docs_2`
- `./vendor/docs` → `docs_3`

The suffix numbering is deterministic based on the order of `-i` arguments.

### 4) Single-Input Backward Compatibility

When **exactly one** input is provided, **no prefix is applied**:

```bash
embeddenator ingest -i ./my_docs -o engram.bin
```

Files are stored with their original relative paths:
```
./my_docs/README.md → stored as: README.md
./my_docs/guide/intro.md → stored as: guide/intro.md
```

This preserves backward compatibility with existing workflows and scripts.

### 5) Implementation: `ingest_directory_with_prefix()`

The internal API function `ingest_directory_with_prefix()` in `src/embrfs.rs` handles namespace prefixing:

```rust
pub fn ingest_directory_with_prefix(
    &mut self,
    base_path: &Path,
    prefix: Option<&str>,
) -> Result<()> {
    for entry in WalkDir::new(base_path) {
        let relative_path = entry.path().strip_prefix(base_path)?;
        let engram_path = if let Some(pfx) = prefix {
            PathBuf::from(pfx).join(relative_path)
        } else {
            relative_path.to_path_buf()
        };
        // ... chunk and bundle into VSA
    }
}
```

The CLI orchestrates multiple calls to this function with appropriate prefixes.

## Alternatives Considered

### Alternative 1: Manual Namespace Specification

**Syntax:**
```bash
embeddenator ingest -i docs:./project_a/docs -i src:./project_a/src -o engram.bin
```

Users explicitly specify the namespace prefix before the path (colon-separated).

**Rejection Rationale:**
- **Poor UX**: Verbose and error-prone. Users must manually invent and track namespace names.
- **Inconsistent Naming**: Different users would choose different conventions, harming documentation portability.
- **Unnecessary Complexity**: For the common case (ingesting top-level directories), automatic prefixing is simpler and sufficient.

### Alternative 2: Hash-Based Prefixes

**Approach:**
Generate a short hash of the input path as the prefix:

```bash
embeddenator ingest -i ./docs -i ./other/docs -o engram.bin
```

Results in prefixes like:
- `./docs` → `a3f9b8_docs`
- `./other/docs` → `7c2d1e_docs`

**Rejection Rationale:**
- **Not Human-Readable**: Query results would show cryptic prefixes like `a3f9b8_docs/README.md`, harming usability.
- **Debugging Difficulty**: Users cannot easily map prefixes back to original input paths.
- **Aesthetic Pollution**: Hash prefixes clutter the namespace without semantic value.

### Alternative 3: Collision Detection Only (No Prefixing)

**Approach:**
Ingest all inputs into a flat namespace. If a collision is detected, abort with an error message.

**Rejection Rationale:**
- **Data Safety Risk**: Silent overwrites (if detection fails) could cause data loss.
- **Poor UX**: Users must manually restructure their directories to avoid collisions.
- **Unclear Merge Behavior**: Users cannot predict which files would collide without trial-and-error.
- **Fragile Workflows**: Adding a new input that happens to collide with an existing path breaks the entire ingest process.

### Alternative 4: Merge All Without Namespacing

**Approach:**
Merge all inputs into a single namespace without prefixing. Files with identical paths are **overwritten** by later inputs (last-write-wins).

**Rejection Rationale:**
- **Silent Overwrites**: Extremely dangerous for data integrity. Users may not realize files were overwritten.
- **Non-Deterministic Behavior**: Changing the order of `-i` arguments changes the final engram content.
- **Violates Principle of Least Surprise**: Users expect all inputs to be preserved, not selectively overwritten.

## Consequences

### Positive Consequences

✅ **Flexible Ingest Workflows**: Users can ingest multiple projects, documentation trees, or selective directories in a single command, streamlining complex workflows.

✅ **Prevents Silent Path Collisions**: Automatic namespace prefixing guarantees that files from different inputs never overwrite each other, ensuring data integrity.

✅ **Human-Readable Prefixes**: Prefixes based on directory basenames are intuitive and easy to understand in query results (e.g., `project_a/docs/README.md`).

✅ **Backward Compatible**: Single-input use cases (the majority of current usage) continue to work without modification. No prefix is applied, preserving existing behavior.

✅ **Automatic Collision Avoidance**: Numeric suffixes handle edge cases (multiple inputs with the same basename) without user intervention or manual namespace specification.

✅ **Deterministic Behavior**: Prefix assignment is deterministic based on input order, ensuring reproducible engram construction.

### Negative Consequences / Trade-offs

⚠️ **Prefix Visible in Query Results**: Users must understand the namespace convention. Query results for "README" will include prefixes like `project_a/docs/README.md` and `project_b/docs/README.md`. This requires minimal user education but is a conceptual overhead.

⚠️ **Potential Verbosity**: For deeply nested directory structures, the full engram path (prefix + relative path) can be lengthy. However, this is a minor UX issue and preferable to path collisions.

⚠️ **No Manual Override**: Users cannot override the automatic prefix with a custom namespace (rejected Alternative 1). This is intentional to keep the API simple, but power users may occasionally want manual control.

## API Design

### CLI Usage

```bash
# Single input (backward compatible, no prefix)
embeddenator ingest -i ./my_docs -o engram.bin

# Multiple inputs (automatic namespace prefixing)
embeddenator ingest -i ./project_a -i ./project_b -i ./shared -o engram.bin

# Collision avoidance (numeric suffixes)
embeddenator ingest -i ./docs -i ./other/docs -i ./vendor/docs -o engram.bin
# → Prefixes: docs, docs_2, docs_3

# Mixed files and directories
embeddenator ingest -i ./docs -i ./specs/overview.md -i ./vendor_docs -o engram.bin
```

### Internal API

The CLI logic in `src/cli.rs` orchestrates multiple calls to `ingest_directory_with_prefix()`:

```rust
pub fn handle_ingest(args: IngestArgs) -> Result<()> {
    let mut resonator = Resonator::new(/* ... */);
    
    let apply_prefix = args.inputs.len() > 1;
    let mut prefix_counts = HashMap::new();
    
    for input_path in &args.inputs {
        let prefix = if apply_prefix {
            let base = input_path.file_name()?.to_string_lossy();
            let count = prefix_counts.entry(base.clone()).or_insert(0);
            *count += 1;
            Some(if *count == 1 {
                base.to_string()
            } else {
                format!("{}_{}", base, count)
            })
        } else {
            None
        };
        
        resonator.ingest_directory_with_prefix(input_path, prefix.as_deref())?;
    }
    
    resonator.save_engram(&args.output)?;
    Ok(())
}
```

## Implementation Notes

- **Code Location**: 
  - CLI argument parsing: [src/cli.rs](src/cli.rs)
  - Namespace prefixing logic: [src/embrfs.rs](src/embrfs.rs) (`ingest_directory_with_prefix()`)
  - End-to-end test: [tests/hierarchical_artifacts_e2e.rs](tests/hierarchical_artifacts_e2e.rs)

- **Manifest Representation**: The hierarchical manifest (JSON) reflects the prefixed paths:
  ```json
  {
    "nodes": {
      "project_a": { "children": ["project_a/docs", "project_a/src"] },
      "project_a/docs": { "chunk_ids": ["project_a/docs/README.md", "project_a/docs/architecture.md"] },
      "project_b": { "children": ["project_b/docs", "project_b/src"] },
      "project_b/docs": { "chunk_ids": ["project_b/docs/README.md", "project_b/docs/api_reference.md"] }
    }
  }
  ```

- **Query Behavior**: Queries against the engram will match files under their prefixed paths. Users can filter by prefix if desired:
  ```bash
  embeddenator query -e engram.bin -q "authentication" --filter-prefix project_a
  ```
  (Assuming future support for prefix filtering.)

## References

- **Related ADRs**:
  - [ADR-009: Deterministic Hierarchical Artifacts](ADR-009-deterministic-hierarchical-artifacts.md) — Establishes the deterministic bundling semantics that namespace prefixing builds upon.
  - [ADR-010: Router-Shard Bounded Indexing](ADR-010-router-shard-bounded-indexing.md) — Sharding mechanism applies independently within each namespace.

- **Implementation**:
  - `src/cli.rs`: `IngestArgs` struct with `Vec<PathBuf>` inputs
  - `src/embrfs.rs`: `ingest_directory_with_prefix()` function
  - `tests/hierarchical_artifacts_e2e.rs`: Multi-input ingestion test cases

- **Release**: Implemented in **v0.3.0** (2026-01-01)

## Conclusion

Multi-input namespace management extends Embeddenator's CLI to support flexible, multi-source ingestion workflows while maintaining data integrity through automatic namespace prefixing. By balancing usability (automatic, human-readable prefixes) with safety (collision avoidance) and backward compatibility (single-input no-prefix behavior), this design enables production-grade multi-project knowledge base construction without sacrificing simplicity or predictability.
