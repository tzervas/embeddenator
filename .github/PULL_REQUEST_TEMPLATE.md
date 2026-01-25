## Description

<!-- Provide a brief description of the changes in this PR -->

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Refactoring (no functional changes)

## Component(s) Affected

<!-- Check all that apply -->

- [ ] embeddenator (main application)
- [ ] embeddenator-vsa
- [ ] embeddenator-fs
- [ ] embeddenator-io
- [ ] embeddenator-obs
- [ ] embeddenator-retrieval
- [ ] embeddenator-interop
- [ ] embeddenator-cli
- [ ] embeddenator-workspace
- [ ] embeddenator-testkit
- [ ] embeddenator-contract-bench
- [ ] embeddenator-integration-tests
- [ ] CI/CD workflows
- [ ] Documentation

## Checklist

### Code Quality

- [ ] My code follows the project's style guidelines (`cargo fmt`)
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] My changes generate no new warnings (`cargo clippy`)

### Testing

- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes (`cargo test`)
- [ ] I have tested the changes manually
- [ ] Benchmarks show no significant performance regression (if applicable)

### Documentation

- [ ] I have updated the documentation to reflect my changes
- [ ] I have updated the CHANGELOG.md (if applicable)
- [ ] All public APIs have doc comments
- [ ] Documentation builds without warnings (`cargo doc`)

### Workspace

- [ ] Version numbers are consistent across dependencies
- [ ] I have run `update_all.sh` to sync all repos
- [ ] Workspace health check passes (`embeddenator-workspace health`)

## Testing Performed

<!-- Describe the testing you performed to verify your changes -->

```bash
# Example:
cargo test --all-features
cargo bench
./embeddenator-workspace/target/release/embeddenator-workspace health
```

## Performance Impact

<!-- Describe any performance implications -->

- [ ] No performance impact
- [ ] Performance improved (provide benchmarks)
- [ ] Performance degraded (explain why acceptable)

**Benchmark Results:**
```
# Paste benchmark results here if applicable
```

## Breaking Changes

<!-- If this is a breaking change, describe the impact and migration path -->

N/A

## Related Issues

<!-- Link to related issues using #issue-number -->

Fixes #
Closes #
Related to #

## Screenshots/Logs

<!-- If applicable, add screenshots or logs to help explain your changes -->

## Additional Notes

<!-- Any additional information that reviewers should know -->

---

## Reviewer Checklist

<!-- For reviewers to complete -->

- [ ] Code review completed
- [ ] Tests are adequate
- [ ] Documentation is clear and complete
- [ ] No security concerns
- [ ] Performance is acceptable
- [ ] CI checks pass
- [ ] Workspace health check passes
