# Hierarchical Engram Format Design

## Overview

The hierarchical engram format extends Embeddenator's flat engram structure to support multi-level encoding for TB-scale datasets. This enables arbitrary scaling while maintaining 100% bit-perfect reconstruction.

## Structure

### HierarchicalManifest

```json
{
  "version": 1,
  "levels": [
    {
      "level": 0,
      "items": [
        {
          "path": "dir1",
          "sub_engram_id": "sub_001"
        }
      ]
    },
    {
      "level": 1,
      "items": [
        {
          "path": "dir1/file1.txt",
          "sub_engram_id": "sub_002"
        }
      ]
    }
  ],
  "sub_engrams": {
    "sub_001": {
      "id": "sub_001",
      "root": {"pos": [1,2,3], "neg": [4,5,6]},
      "chunk_count": 50,
      "children": ["sub_002"]
    },
    "sub_002": {
      "id": "sub_002",
      "root": {"pos": [7,8,9], "neg": [10,11,12]},
      "chunk_count": 10,
      "children": []
    }
  }
}
```

### Key Components

- **version**: Format version for future compatibility
- **levels**: Hierarchical levels from root (0) to leaves (N)
- **sub_engrams**: Map of sub-engram IDs to their data
- **ManifestLevel**: Contains items at that hierarchy level
- **ManifestItem**: Links path to sub-engram ID
- **SubEngram**: Contains root vector, chunk count, and child references

## Encoding Process

1. **Level 0**: Encode individual files into small bundles (≤1000 chunks)
2. **Level 1**: Bundle directory contents with path role binding
3. **Higher Levels**: Recursively bundle sub-engrams
4. **Root**: Final bundle of top-level sub-engrams

## Backward Compatibility

The `UnifiedManifest` enum supports both formats:

```rust
pub enum UnifiedManifest {
    Flat(Manifest),                    // v0.2.0 format
    Hierarchical(HierarchicalManifest) // v0.3.0+ format
}
```

Existing engrams deserialize as `Flat`, new hierarchical ones as `Hierarchical`.

## Size Limits

- **Max chunks per sub-engram**: 1000 (configurable)
- **Max sub-engrams per level**: 1000
- **Max hierarchy depth**: 30 levels

## Benefits

- **Scalability**: Support PB datasets with controlled noise
- **Performance**: Parallel encoding/decoding of sub-engrams
- **Flexibility**: Mix flat and hierarchical in same system
- **Compatibility**: Seamless migration from flat format

## Implementation Notes

- Sub-engrams stored separately for distributed processing
- Root vectors enable algebraic queries across hierarchy
- Manifest guides level-by-level reconstruction
- Path role binding uses permutation for orthogonality