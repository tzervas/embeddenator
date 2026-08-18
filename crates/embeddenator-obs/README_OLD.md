# embeddenator-obs

Extracted component from [Embeddenator](https://github.com/tzervas/embeddenator) monorepo.

## Status

**Phase 2A Component Extraction** - Initial split from embeddenator core.

## Usage

```toml
[dependencies]
embeddenator-obs = { git = "https://github.com/tzervas/embeddenator-obs", tag = "v0.1.0" }
```

## Development

```bash
# Local development with other Embeddenator components
cargo build
cargo test

# For cross-repo work, use Cargo patches:
# Add to Cargo.toml:
# [patch."https://github.com/tzervas/embeddenator-obs"]
# embeddenator-obs = { path = "../embeddenator-obs" }
```

## Architecture

See [ADR-016](https://github.com/tzervas/embeddenator/blob/main/docs/adr/ADR-016-component-decomposition.md) for component decomposition rationale.

## License

MIT
