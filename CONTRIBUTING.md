# Contributing to Embeddenator

## Branch Strategy

```
main (release)
  ↑
  PR (with version tag)
  ↑
dev (working)
  ↑
  PR
  ↑
feature/* or fix/* (your work)
```

### Branches

| Branch | Purpose | Protected |
|--------|---------|-----------|
| `main` | Release branch - versioned releases only | Yes |
| `dev` | Working branch - all PRs merge here first | Yes |
| `feature/*` | New features | No |
| `fix/*` | Bug fixes | No |

### Workflow

1. **Start work**: Create branch from `dev`
   ```bash
   git checkout dev
   git pull origin dev
   git checkout -b feature/my-feature
   ```

2. **Do work**: Make commits, push to origin
   ```bash
   git add .
   git commit -m "feat: add my feature"
   git push -u origin feature/my-feature
   ```

3. **Create PR to dev**: Open PR targeting `dev` branch
   - PRs require review
   - CI must pass
   - Squash merge preferred

4. **Release to main**: When ready for release
   - Create PR from `dev` → `main`
   - Include version bump
   - Tag release after merge

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `refactor`: Code refactoring
- `test`: Adding tests
- `chore`: Maintenance

Examples:
```
feat(vsa): add ReversibleVSAEncoder with 94% accuracy
fix(fs): handle signal interrupts gracefully
docs: update gap analysis with benchmark results
```

## Development Setup

```bash
# Clone
git clone https://github.com/tzervas/embeddenator.git
cd embeddenator

# Switch to dev
git checkout dev

# Build
cargo build --release

# Test
cargo test --all-features --workspace
```

## Code Standards

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add tests for new functionality
- Update documentation as needed

## Issue Labels

| Label | Meaning |
|-------|---------|
| `priority: critical` | Blocking for release |
| `priority: high` | Important for release |
| `priority: medium` | Nice to have |
| `area: vsa` | VSA core |
| `area: fs` | Filesystem |
| `area: performance` | Performance work |
| `type: feature` | New feature |
| `type: refactor` | Refactoring |
| `epic` | Large multi-issue effort |
