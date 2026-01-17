# Archive Plan for Monolithic Embeddenator Repository

**Status:** ACTIVE  
**Created:** January 16, 2026  
**Target Archive Date:** Q2 2026 (v0.21.0)  
**Target Removal Date:** Q3 2026 (v1.0.0)

---

## Executive Summary

This document outlines the formal archival and eventual removal plan for the deprecated monolithic `embeddenator` repository. All functionality has been successfully migrated to 10 component repositories, and this archive plan ensures a smooth transition for existing users.

---

## Timeline

### Phase 1: Deprecation (v0.20.0-alpha.1 - Current)
**Status:** ✅ COMPLETE  
**Date:** January 16, 2026

**Completed Actions:**
- ✅ Added deprecation notices to README.md
- ✅ Created DEPRECATED.md with comprehensive migration guide
- ✅ Updated Cargo.toml with deprecation metadata
- ✅ Added deprecation attributes to Rust code
- ✅ Created ARCHIVE_PLAN.md (this document)
- ✅ Updated workspace documentation

**Ongoing Support:**
- ⚠️ **No new features** - Only critical bug fixes
- ⚠️ **Security fixes only** - Critical CVEs will be patched
- ⚠️ **No API changes** - APIs frozen at v0.20.0-alpha.1
- ℹ️ **Documentation updates** - Redirects and migration guides

### Phase 2: Final Release & Read-Only Archive (v0.21.0)
**Status:** 🕐 PLANNED  
**Target Date:** Q2 2026 (April-June 2026)  
**Duration:** 1 week preparation + release

**Planned Actions:**

#### Pre-Release (1 week before)
- [ ] Final security audit and vulnerability scan
- [ ] Final test suite run to ensure stability
- [ ] Update all deprecation warnings with final dates
- [ ] Create final migration guide with v0.21.0 specifics
- [ ] Notify all known users via:
  - GitHub issue announcement
  - Discussion post
  - README banner update
  - Release notes

#### Release Day
- [ ] Tag v0.21.0 with comprehensive release notes
- [ ] Publish final crate version to crates.io
- [ ] Mark repository as archived in GitHub settings
- [ ] Enable read-only mode (disable issues, PRs, wiki)
- [ ] Update README with prominent "ARCHIVED" banner
- [ ] Add .github/ARCHIVED.md file
- [ ] Update all documentation links to component repos

#### Post-Release
- [ ] Archive GitHub Discussions (read-only)
- [ ] Close all open issues with migration notices
- [ ] Close all open pull requests
- [ ] Update CI/CD workflows to minimal status checks only
- [ ] Remove write permissions for all contributors

**Repository State:**
- 🔒 **Read-only** - No new commits accepted
- 📦 **Stable** - v0.21.0 is final stable release
- 🚫 **No issues/PRs** - All redirected to component repos
- ℹ️ **Documentation available** - Historical reference only

### Phase 3: Removal from Workspace (v1.0.0)
**Status:** 🕐 PLANNED  
**Target Date:** Q3 2026 (July-September 2026)  
**Duration:** 2 weeks preparation + removal

**Planned Actions:**

#### Pre-Removal (2 weeks before)
- [ ] Final notification to users
- [ ] Verify all component repos are production-ready
- [ ] Ensure all migration guides are complete
- [ ] Create permanent redirect pages
- [ ] Archive documentation to separate branch
- [ ] Update workspace Cargo.toml to remove embeddenator

#### Removal Day
- [ ] Remove `embeddenator/` directory from workspace
- [ ] Update workspace README to remove references
- [ ] Update CI/CD to exclude embeddenator
- [ ] Create `ARCHIVED_REPOS.md` with links to archived code
- [ ] Update all examples to use component repos only
- [ ] Tag workspace as v1.0.0 (first major release without monolith)

#### Post-Removal
- [ ] GitHub repository remains visible (archived)
- [ ] Git history preserved in separate branch
- [ ] Documentation redirects remain active
- [ ] Component repositories promoted as canonical

**Final State:**
- 🗑️ **Removed from workspace** - No longer part of active development
- 📚 **Historical archive** - Accessible via Git history
- 🔗 **Redirects active** - All links point to component repos
- ✅ **Migration complete** - All users transitioned

---

## Technical Details

### Repository Archival Process

#### GitHub Settings Changes (v0.21.0)
```bash
# Mark repository as archived
gh repo edit tzervas/embeddenator --archived

# Disable features
gh repo edit tzervas/embeddenator \
  --enable-issues=false \
  --enable-projects=false \
  --enable-wiki=false
```

#### File Changes (v0.21.0)
```bash
# Add archive marker
touch .archived
echo "v0.21.0 - Archived $(date)" > .archived

# Update README with archive banner
cat > README.md << 'EOF'
# 🔒 ARCHIVED: Embeddenator Monolithic Repository

**This repository has been ARCHIVED and is READ-ONLY.**

See [DEPRECATED.md](DEPRECATED.md) for migration information.

**Use component repositories instead:**
- [embeddenator-vsa](../embeddenator-vsa/)
- [embeddenator-fs](../embeddenator-fs/)
- [embeddenator-retrieval](../embeddenator-retrieval/)
- [More components...](../README.md)

EOF
```

#### Cargo.toml Updates (v0.21.0)
```toml
[package]
name = "embeddenator"
version = "0.21.0"  # Final version
edition = "2021"
# Add deprecation metadata
metadata.deprecated = true
metadata.archived = true
metadata.archive_date = "2026-06-01"
```

### Workspace Changes

#### Remove from Workspace Cargo.toml (v1.0.0)
```toml
[workspace]
members = [
    # "embeddenator",  # REMOVED - See ARCHIVED_REPOS.md
    "embeddenator-vsa",
    "embeddenator-fs",
    "embeddenator-retrieval",
    "embeddenator-io",
    "embeddenator-obs",
    "embeddenator-interop",
    "embeddenator-cli",
    "embeddenator-testkit",
    "embeddenator-workspace",
    "embeddenator-contract-bench",
    "embeddenator-integration-tests",
]
```

#### Directory Removal (v1.0.0)
```bash
# Before removal, create archive branch
git checkout -b archive/embeddenator-monolith
git add embeddenator/
git commit -m "Archive: Final snapshot of monolithic embeddenator"
git push origin archive/embeddenator-monolith

# Return to main and remove
git checkout main
git rm -rf embeddenator/
git commit -m "Remove deprecated embeddenator monolithic repository"
```

---

## Communication Plan

### Notification Schedule

#### 6 Months Before Archival (v0.20.0-alpha.1 - January 2026)
- ✅ Deprecation announcement in README
- ✅ DEPRECATED.md created with migration guide
- ✅ GitHub Discussion post
- ✅ Update documentation with warnings

#### 3 Months Before Archival (March 2026)
- [ ] Email notification to known users
- [ ] Update all crate documentation on crates.io
- [ ] Blog post on project website (if exists)
- [ ] Social media announcement

#### 1 Month Before Archival (May 2026)
- [ ] Final migration reminder
- [ ] GitHub issue with checklist for users
- [ ] Update CI to warn on every build
- [ ] Add compile-time deprecation warnings

#### Archival Day (June 2026)
- [ ] v0.21.0 release announcement
- [ ] Repository archived notification
- [ ] All documentation updated
- [ ] Component repos promoted

#### 1 Month Before Removal (August 2026)
- [ ] Final removal warning
- [ ] Verify all users have migrated
- [ ] Check component repo stability
- [ ] Prepare removal checklist

#### Removal Day (September 2026)
- [ ] v1.0.0 workspace release
- [ ] Repository removed from workspace
- [ ] Archive branch created
- [ ] Documentation updated

---

## Migration Support

### Resources Available During Transition

1. **Documentation**
   - [DEPRECATED.md](DEPRECATED.md) - Comprehensive deprecation guide
   - [../MIGRATION_GUIDE.md](../MIGRATION_GUIDE.md) - Step-by-step migration
   - Component README files - Individual component documentation

2. **Support Channels**
   - GitHub Discussions - Community support (until v0.21.0)
   - Component repository issues - Specific component questions
   - Documentation website - Guides and examples

3. **Migration Tools**
   - Automated migration scripts (if developed)
   - Code transformation tools (if needed)
   - Test compatibility suites

### Common Migration Scenarios

#### Scenario 1: Full Monolith User
**User Type:** Uses all features of monolithic embeddenator  
**Migration Path:** Switch to all component dependencies  
**Estimated Time:** 1-2 hours  
**Difficulty:** Easy (mostly import changes)

#### Scenario 2: Partial Feature User
**User Type:** Uses only VSA and filesystem features  
**Migration Path:** Depend on embeddenator-vsa and embeddenator-fs only  
**Estimated Time:** 30 minutes  
**Difficulty:** Very Easy

#### Scenario 3: Heavy Customization
**User Type:** Has custom modifications to monolithic code  
**Migration Path:** Extract customizations to separate crate, depend on components  
**Estimated Time:** 4-8 hours  
**Difficulty:** Moderate

---

## Risk Mitigation

### Identified Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|---------|-----------|
| Users don't migrate in time | Medium | High | Multiple notifications, long timeline |
| Breaking changes in components | Low | High | Semantic versioning, compatibility tests |
| Loss of unique functionality | Very Low | High | Thorough migration validation |
| Documentation gaps | Low | Medium | Comprehensive migration guides |
| Build failures | Low | Medium | Test all component combinations |

### Contingency Plans

#### If Users Don't Migrate
- Extend archival timeline by 3 months
- Provide one-on-one migration support
- Create automated migration tool

#### If Critical Bug Found Post-Archive
- Create hotfix branch from v0.21.0
- Release v0.21.1 with fix only
- Notify users via GitHub releases
- Extend support window if necessary

#### If Component Repository Issues
- Delay removal until issues resolved
- Backport fixes to monolithic if critical
- Adjust timeline as needed

---

## Success Criteria

### Phase 1 Success (Deprecation) ✅
- ✅ All deprecation notices in place
- ✅ Migration guide created and published
- ✅ Component repositories production-ready
- ✅ No functionality gaps identified

### Phase 2 Success (Archive)
- [ ] v0.21.0 released successfully
- [ ] Repository marked as archived
- [ ] All issues/PRs closed with migration notices
- [ ] Zero critical bugs in final release
- [ ] >80% of known users migrated

### Phase 3 Success (Removal)
- [ ] Repository removed from workspace
- [ ] Archive branch created and accessible
- [ ] All documentation redirects working
- [ ] Component repositories stable
- [ ] >95% of users migrated
- [ ] Zero regression reports

---

## Post-Archival Maintenance

### What Remains Active

1. **GitHub Repository**
   - Remains visible and accessible
   - Read-only mode (no new commits)
   - Git history preserved
   - Tagged releases available

2. **Documentation**
   - Archived documentation remains accessible
   - Redirects to component repos active
   - Migration guides maintained

3. **Crates.io**
   - v0.21.0 remains published
   - Yank flag NOT set (for historical builds)
   - Deprecation notice in description

### What is Removed

1. **Active Development**
   - No new features
   - No bug fixes (except critical security)
   - No pull request reviews
   - No issue triage

2. **CI/CD**
   - GitHub Actions workflows disabled
   - Automated tests no longer run
   - Benchmarks archived

3. **Support**
   - No official support
   - Issues redirected to component repos
   - Community support only

---

## Monitoring and Metrics

### Tracking Migration Progress

#### Metrics to Track
- Download counts on crates.io (monolithic vs components)
- GitHub star migrations
- Open issues in monolithic repo
- User questions about migration
- Component repository adoption rates

#### Monitoring Dashboard
```
Embeddenator Migration Metrics (Updated Weekly)

Monolithic Repository (embeddenator):
- Downloads (last 90 days): [TBD]
- Open Issues: [TBD]
- Active Users: [TBD]

Component Repositories:
- Total Downloads: [TBD]
- Active Development: [TBD]
- Migration Completion: [TBD]%
```

---

## Appendices

### Appendix A: File Changes Checklist

**v0.20.0-alpha.1 (Deprecation):**
- ✅ `DEPRECATED.md` - Created
- ✅ `ARCHIVE_PLAN.md` - Created (this file)
- ✅ `README.md` - Updated with deprecation banner
- ✅ `Cargo.toml` - Updated with deprecation metadata
- ✅ `src/lib.rs` - Added deprecation attributes

**v0.21.0 (Archive):**
- [ ] `.archived` - Create archive marker file
- [ ] `.github/ARCHIVED.md` - Archive notice
- [ ] `README.md` - Replace with archive banner
- [ ] `Cargo.toml` - Mark as archived
- [ ] All CI workflows - Disable or minimal

**v1.0.0 (Removal):**
- [ ] Remove `embeddenator/` directory
- [ ] Create `ARCHIVED_REPOS.md` in workspace root
- [ ] Update workspace `README.md`
- [ ] Update workspace CI/CD

### Appendix B: Communication Templates

#### Deprecation Announcement Template
```markdown
# Embeddenator Monolithic Repository Deprecated

We're excited to announce that Embeddenator has evolved into a modern
component-based architecture!

The monolithic repository is now deprecated in favor of 10 independent
component repositories that offer better modularity and faster development.

**Timeline:**
- Now (v0.20.0): Deprecated, migration encouraged
- Q2 2026 (v0.21.0): Archived, read-only
- Q3 2026 (v1.0.0): Removed from workspace

**Migration Guide:** See DEPRECATED.md for details

**Component Repositories:** [List links]
```

#### Archive Announcement Template
```markdown
# Embeddenator Monolithic Repository Archived

As of v0.21.0, the monolithic embeddenator repository is officially
ARCHIVED and marked read-only.

All development has moved to component repositories. This repository
will remain accessible for historical reference but will not receive
updates.

**If you haven't migrated yet:** See DEPRECATED.md for migration guide.

**Component Repositories:** [List links]
```

### Appendix C: References

- [DEPRECATED.md](DEPRECATED.md) - Deprecation notice and migration guide
- [../MIGRATION_GUIDE.md](../MIGRATION_GUIDE.md) - Comprehensive migration documentation
- [../MIGRATION_COMPLETE.md](../MIGRATION_COMPLETE.md) - Migration completion report
- Component repository READMEs

---

**Document Version:** 1.0  
**Last Updated:** January 16, 2026  
**Next Review:** March 2026  
**Owner:** Tyler Zervas <tz-dev@vectorweight.com>
