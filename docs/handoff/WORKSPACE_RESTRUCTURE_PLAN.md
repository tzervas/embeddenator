# Embeddenator Workspace Restructure Plan

**Date:** January 4, 2026  
**Purpose:** Clean up project structure for better organization  
**Status:** PLANNING (Pending Approval)

---

## Executive Summary

Restructure the Embeddenator ecosystem from a confusing nested structure to a clean, flat workspace layout where each repo is a direct sibling under a common non-git workspace directory.

---

## 1. Current State (Problems)

### Directory Structure
```
~/Documents/projects/
├── embeddenator/                    # Sister project space (NOT a git repo ✓)
│   ├── embeddenator/               # ❌ CONFUSING: Clone of main repo nested here
│   ├── embeddenator-vsa/           # ✓ Sister repo
│   ├── embeddenator-retrieval/     # ✓ Sister repo  
│   ├── embeddenator-fs/            # ✓ Sister repo
│   ├── embeddenator-interop/       # ✓ Sister repo
│   ├── embeddenator-io/            # ✓ Sister repo
│   ├── embeddenator-obs/           # ✓ Sister repo
│   ├── embeddenator-cli/           # ⚠️ Has different code than crates/
│   ├── embeddenator-agent-mcp/     # ✓ Sister repo
│   ├── embeddenator-context-mcp/   # ✓ Sister repo
│   ├── embeddenator-security-mcp/  # ✓ Sister repo
│   ├── embeddenator-webpuppet/     # ✓ Sister repo
│   ├── embeddenator-webpuppet-mcp/ # ✓ Sister repo
│   ├── embeddenator-contract-bench/# ✓ Sister repo
│   ├── embeddenator-testkit/       # ✓ Sister repo
│   └── embeddenator-workspace/     # ✓ Sister repo
│
└── github/
    └── embeddenator/               # ❌ DUPLICATE: Main repo also here
        └── crates/
            ├── embeddenator/       # Core library
            ├── embeddenator-cli/   # ⚠️ Has MORE code than sister repo (1,323 vs 715 LOC)
            └── embeddenator-screen-mcp/  # Empty placeholder
```

### Problems
1. **Two copies of main repo** - `github/embeddenator` AND `embeddenator/embeddenator`
2. **Confusing nesting** - `embeddenator/embeddenator/` is weird
3. **CLI code divergence** - Two different implementations
4. **crates/ inside github/** - Creates workspace-within-workspace confusion
5. **Path dependency hell** - Hard to manage relative paths

---

## 2. Target State (Clean)

### New Directory Structure
```
~/Documents/projects/embeddenator/          # Workspace root (NOT a git repo)
│
├── .vscode/                                # Workspace-wide VS Code settings
│   ├── settings.json
│   ├── launch.json
│   └── tasks.json
│
├── embeddenator.code-workspace             # VS Code multi-root workspace file
├── Cargo.toml                              # Virtual workspace manifest (optional)
├── README.md                               # Workspace overview
│
├── embeddenator/                           # Core orchestration repo (github.com/tzervas/embeddenator)
│   ├── Cargo.toml                          # Points to component crates via path deps
│   ├── src/                                # Main binary/integration code
│   ├── benches/
│   ├── tests/
│   └── docs/
│
├── embeddenator-vsa/                       # VSA component (v0.2.0)
├── embeddenator-retrieval/                 # Retrieval component (v0.2.0)
├── embeddenator-fs/                        # Filesystem component (v0.2.0)
├── embeddenator-interop/                   # Interop component (v0.2.0)
├── embeddenator-io/                        # IO component (v0.2.0)
├── embeddenator-obs/                       # Observability component (v0.2.0)
├── embeddenator-cli/                       # CLI component (v0.2.0) - FIXED with proper code
│
├── embeddenator-agent-mcp/                 # MCP: Agent
├── embeddenator-context-mcp/               # MCP: Context
├── embeddenator-security-mcp/              # MCP: Security
├── embeddenator-webpuppet/                 # Browser automation lib
├── embeddenator-webpuppet-mcp/             # MCP: Webpuppet
│
├── embeddenator-contract-bench/            # Benchmarking tool
├── embeddenator-testkit/                   # Testing utilities
└── embeddenator-workspace/                 # Workspace management tool
```

### Benefits
- ✅ **Flat structure** - All repos are siblings, no nesting confusion
- ✅ **Single source of truth** - One location for each repo
- ✅ **VS Code workspace** - Unified settings, debugging, tasks
- ✅ **Clear boundaries** - Each git repo is isolated
- ✅ **Easy path deps** - All use `path = "../embeddenator-xxx"`

---

## 3. Migration Plan

### Phase M1: Preparation (5 min)

```bash
# Create backup
cp -r ~/Documents/projects/embeddenator ~/Documents/projects/embeddenator.bak.$(date +%Y%m%d)

# Verify all repos have clean working trees
for repo in ~/Documents/projects/embeddenator/embeddenator-*/; do
    cd "$repo" && git status --short
done
```

### Phase M2: Remove Duplicates (10 min)

| Action | Source | Target | Notes |
|--------|--------|--------|-------|
| **DELETE** | `~/Documents/projects/embeddenator/embeddenator/` | - | Duplicate of main repo |
| **MOVE** | `~/Documents/projects/github/embeddenator/` | `~/Documents/projects/embeddenator/embeddenator/` | Main repo to workspace |
| **DELETE** | `~/Documents/projects/github/` | - | If empty after move |

```bash
# Step 1: Remove the nested duplicate
rm -rf ~/Documents/projects/embeddenator/embeddenator

# Step 2: Move main repo from github/ to workspace
mv ~/Documents/projects/github/embeddenator ~/Documents/projects/embeddenator/embeddenator

# Step 3: Clean up github/ if empty
rmdir ~/Documents/projects/github 2>/dev/null || echo "github/ not empty, keeping"
```

### Phase M3: Fix CLI (20 min)

The `crates/embeddenator-cli/` has the correct, more complete code (1,323 LOC).
The sister `embeddenator-cli/` has incomplete code (715 LOC).

```bash
# Step 1: Backup current sister CLI
mv ~/Documents/projects/embeddenator/embeddenator-cli \
   ~/Documents/projects/embeddenator/embeddenator-cli.old

# Step 2: Move crates/embeddenator-cli to become the new sister repo
mv ~/Documents/projects/embeddenator/embeddenator/crates/embeddenator-cli \
   ~/Documents/projects/embeddenator/embeddenator-cli

# Step 3: Initialize as git repo (crates/ version wasn't a git repo)
cd ~/Documents/projects/embeddenator/embeddenator-cli
git init
git remote add origin https://github.com/tzervas/embeddenator-cli.git
# Copy .gitignore, LICENSE from old or create new
cp ../embeddenator-cli.old/.gitignore . 2>/dev/null || echo "target/" > .gitignore
cp ../embeddenator-cli.old/LICENSE . 2>/dev/null || cp ../embeddenator/LICENSE .

# Step 4: Commit and tag
git add .
git commit -m "feat: Initialize embeddenator-cli with full extraction (1,323 LOC)

Extracted from embeddenator monorepo crates/embeddenator-cli/
- 7 main commands: Ingest, Extract, Query, QueryText, BundleHier, Mount, Update
- 4 update subcommands: Add, Remove, Modify, Compact
- Modular structure: commands/, utils/ directories
- Full API surface preserved"

git tag v0.2.0

# Step 5: Push (may need to force if repo exists with different history)
git push -u origin main --force
git push origin v0.2.0

# Step 6: Clean up old
rm -rf ~/Documents/projects/embeddenator/embeddenator-cli.old
```

### Phase M4: Clean Up Main Repo crates/ (5 min)

After CLI is moved, clean up the main repo's crates/ directory:

```bash
cd ~/Documents/projects/embeddenator/embeddenator

# Remove the now-empty CLI crate directory
rm -rf crates/embeddenator-cli

# Remove empty screen-mcp placeholder
rm -rf crates/embeddenator-screen-mcp

# crates/embeddenator/ stays - it's the core library
ls -la crates/
# Should only show: embeddenator/
```

### Phase M5: Create Workspace Files (15 min)

#### 5.1 VS Code Workspace File

```bash
cat > ~/Documents/projects/embeddenator/embeddenator.code-workspace << 'EOF'
{
  "folders": [
    { "name": "📦 embeddenator (core)", "path": "embeddenator" },
    { "name": "🧠 vsa", "path": "embeddenator-vsa" },
    { "name": "🔍 retrieval", "path": "embeddenator-retrieval" },
    { "name": "📁 fs", "path": "embeddenator-fs" },
    { "name": "🔗 interop", "path": "embeddenator-interop" },
    { "name": "💾 io", "path": "embeddenator-io" },
    { "name": "📊 obs", "path": "embeddenator-obs" },
    { "name": "⌨️ cli", "path": "embeddenator-cli" },
    { "name": "🤖 agent-mcp", "path": "embeddenator-agent-mcp" },
    { "name": "📝 context-mcp", "path": "embeddenator-context-mcp" },
    { "name": "🔒 security-mcp", "path": "embeddenator-security-mcp" },
    { "name": "🌐 webpuppet", "path": "embeddenator-webpuppet" },
    { "name": "🌐 webpuppet-mcp", "path": "embeddenator-webpuppet-mcp" },
    { "name": "📏 contract-bench", "path": "embeddenator-contract-bench" },
    { "name": "🧪 testkit", "path": "embeddenator-testkit" },
    { "name": "🗂️ workspace", "path": "embeddenator-workspace" }
  ],
  "settings": {
    "rust-analyzer.linkedProjects": [
      "embeddenator/Cargo.toml",
      "embeddenator-vsa/Cargo.toml",
      "embeddenator-retrieval/Cargo.toml",
      "embeddenator-fs/Cargo.toml",
      "embeddenator-interop/Cargo.toml",
      "embeddenator-io/Cargo.toml",
      "embeddenator-obs/Cargo.toml",
      "embeddenator-cli/Cargo.toml",
      "embeddenator-agent-mcp/Cargo.toml",
      "embeddenator-context-mcp/Cargo.toml",
      "embeddenator-security-mcp/Cargo.toml",
      "embeddenator-webpuppet/Cargo.toml",
      "embeddenator-webpuppet-mcp/Cargo.toml",
      "embeddenator-contract-bench/Cargo.toml",
      "embeddenator-testkit/Cargo.toml",
      "embeddenator-workspace/Cargo.toml"
    ],
    "rust-analyzer.cargo.buildScripts.enable": true,
    "rust-analyzer.procMacro.enable": true,
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "[rust]": {
      "editor.defaultFormatter": "rust-lang.rust-analyzer"
    }
  },
  "extensions": {
    "recommendations": [
      "rust-lang.rust-analyzer",
      "tamasfe.even-better-toml",
      "serayuzgur.crates",
      "vadimcn.vscode-lldb"
    ]
  }
}
EOF
```

#### 5.2 Workspace README

```bash
cat > ~/Documents/projects/embeddenator/README.md << 'EOF'
# Embeddenator Workspace

Multi-repository workspace for the Embeddenator ecosystem.

## Quick Start

```bash
# Open in VS Code
code embeddenator.code-workspace
```

## Repository Layout

| Repository | Description | Version |
|------------|-------------|---------|
| `embeddenator/` | Core library and orchestration | v0.20.0 |
| `embeddenator-vsa/` | Vector Symbolic Architecture | v0.2.0 |
| `embeddenator-retrieval/` | Retrieval and search | v0.2.0 |
| `embeddenator-fs/` | Filesystem operations | v0.2.0 |
| `embeddenator-interop/` | Interoperability layer | v0.2.0 |
| `embeddenator-io/` | I/O operations | v0.2.0 |
| `embeddenator-obs/` | Observability | v0.2.0 |
| `embeddenator-cli/` | Command-line interface | v0.2.0 |
| `embeddenator-*-mcp/` | MCP servers | v0.1.0-alpha |
| `embeddenator-testkit/` | Testing utilities | v0.1.1 |
| `embeddenator-contract-bench/` | Benchmarking | v0.2.1 |
| `embeddenator-workspace/` | Workspace management | v0.1.0-alpha |

## Building All Projects

```bash
for repo in embeddenator*/; do
    echo "=== Building $repo ===" 
    cd "$repo" && cargo build && cd ..
done
```

## Running Tests

```bash
for repo in embeddenator*/; do
    echo "=== Testing $repo ===" 
    cd "$repo" && cargo test && cd ..
done
```
EOF
```

### Phase M6: Update Path Dependencies (15 min)

Update all Cargo.toml files to use the new flat structure:

**Pattern:** `path = "../embeddenator-xxx"` (all siblings)

```bash
# Check all path dependencies
grep -r "path = " ~/Documents/projects/embeddenator/*/Cargo.toml | grep -v target
```

Most should already be correct. Update any that use nested paths.

### Phase M7: Validation (15 min)

```bash
cd ~/Documents/projects/embeddenator

# Test 1: All repos exist and are git repos
for dir in embeddenator*/; do
    if [ -d "$dir/.git" ]; then
        echo "✅ $dir is a git repo"
    else
        echo "❌ $dir is NOT a git repo"
    fi
done

# Test 2: All repos build
for dir in embeddenator*/; do
    echo "=== Building $dir ==="
    cd "$dir" && cargo build 2>&1 | tail -1 && cd ..
done

# Test 3: Check remotes
for dir in embeddenator*/; do
    echo "=== $dir remote ==="
    cd "$dir" && git remote -v | head -1 && cd ..
done

# Test 4: Open workspace in VS Code
code embeddenator.code-workspace
```

---

## 4. Summary of Changes

### Files/Directories to DELETE

| Path | Reason |
|------|--------|
| `~/Documents/projects/embeddenator/embeddenator/` | Duplicate of main repo |
| `~/Documents/projects/github/embeddenator/` | Moved to workspace |
| `~/Documents/projects/embeddenator/embeddenator-cli.old/` | After CLI fix |
| `crates/embeddenator-cli/` | Moved to sister repo |
| `crates/embeddenator-screen-mcp/` | Empty placeholder |

### Files/Directories to MOVE

| From | To | Notes |
|------|-----|-------|
| `~/Documents/projects/github/embeddenator/` | `~/Documents/projects/embeddenator/embeddenator/` | Main repo |
| `crates/embeddenator-cli/` | `~/Documents/projects/embeddenator/embeddenator-cli/` | Reinit as git repo |

### Files to CREATE

| Path | Purpose |
|------|---------|
| `~/Documents/projects/embeddenator/embeddenator.code-workspace` | VS Code workspace |
| `~/Documents/projects/embeddenator/README.md` | Workspace overview |
| `~/Documents/projects/embeddenator/.vscode/` | Shared VS Code settings (optional) |

---

## 5. Execution Checklist

### Pre-flight
- [ ] All repos have clean working trees (no uncommitted changes)
- [ ] All repos pushed to GitHub
- [ ] Backup created

### Phase M1: Preparation
- [ ] Create backup
- [ ] Verify clean repos

### Phase M2: Remove Duplicates
- [ ] Delete nested `embeddenator/embeddenator/`
- [ ] Move `github/embeddenator/` to workspace
- [ ] Clean up empty directories

### Phase M3: Fix CLI
- [ ] Backup old CLI
- [ ] Move crates/embeddenator-cli to workspace
- [ ] Initialize git repo
- [ ] Add remote
- [ ] Commit with proper message
- [ ] Tag v0.2.0
- [ ] Push to GitHub
- [ ] Delete old CLI backup

### Phase M4: Clean Up crates/
- [ ] Remove empty crates/embeddenator-cli
- [ ] Remove empty crates/embeddenator-screen-mcp
- [ ] Verify crates/embeddenator/ exists

### Phase M5: Create Workspace Files
- [ ] Create embeddenator.code-workspace
- [ ] Create README.md
- [ ] Create .vscode/ settings (optional)

### Phase M6: Update Dependencies
- [ ] Check all path deps use `../` prefix
- [ ] Fix any incorrect paths

### Phase M7: Validation
- [ ] All repos are git repos
- [ ] All repos build
- [ ] All repos have correct remotes
- [ ] VS Code workspace opens correctly
- [ ] rust-analyzer works

### Post-migration
- [ ] Update SPLIT_TRACKER.md
- [ ] Commit documentation updates
- [ ] Push all changes

---

## 6. Rollback Plan

If anything goes wrong:

```bash
# Restore from backup
rm -rf ~/Documents/projects/embeddenator
mv ~/Documents/projects/embeddenator.bak.YYYYMMDD ~/Documents/projects/embeddenator
```

---

## 7. Time Estimate

| Phase | Duration |
|-------|----------|
| M1: Preparation | 5 min |
| M2: Remove Duplicates | 10 min |
| M3: Fix CLI | 20 min |
| M4: Clean Up crates/ | 5 min |
| M5: Create Workspace | 15 min |
| M6: Update Dependencies | 15 min |
| M7: Validation | 15 min |
| **Total** | **~85 min** |

---

## 8. Approval

**Ready to proceed?**

- [ ] Plan reviewed
- [ ] Backup strategy acceptable
- [ ] Time estimate acceptable

**Approve to begin execution.**
