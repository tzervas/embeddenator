# GitHub Projects Setup for Embeddenator Phase 2A

**Date:** 2026-01-04  
**Purpose:** Coordinate Phase 2A component extraction across repos and branches

---

## Overview

GitHub Projects provides a centralized dashboard to track:
- Component extraction progress (6 components)
- Cross-repo dependencies
- Issue linkage (#18-#24)
- Branch coordination (feat/extract-*)

## Setup Instructions

### 1. Authenticate GitHub CLI with Project Scopes

```bash
gh auth refresh -s read:project -s project
```

Follow browser authentication flow.

### 2. Create Phase 2A Project

```bash
gh project create \
  --owner tzervas \
  --title "Phase 2A: Component Extraction" \
  --description "Track decomposition of embeddenator monorepo into 6 core components"
```

### 3. Configure Project Board

**Columns:**
- 📋 **Backlog** - Not started
- ⏳ **In Progress** - Active work
- 🔍 **Review** - Ready for integration testing
- ✅ **Complete** - Released and documented

### 4. Link Issues to Project

```bash
# Get project number from creation output
PROJECT_NUMBER=<project_id>

# Link all Phase 2A issues
gh project item-add $PROJECT_NUMBER --owner tzervas --url https://github.com/tzervas/embeddenator/issues/18
gh project item-add $PROJECT_NUMBER --owner tzervas --url https://github.com/tzervas/embeddenator/issues/19
gh project item-add $PROJECT_NUMBER --owner tzervas --url https://github.com/tzervas/embeddenator/issues/20
gh project item-add $PROJECT_NUMBER --owner tzervas --url https://github.com/tzervas/embeddenator/issues/21
gh project item-add $PROJECT_NUMBER --owner tzervas --url https://github.com/tzervas/embeddenator/issues/22
gh project item-add $PROJECT_NUMBER --owner tzervas --url https://github.com/tzervas/embeddenator/issues/23
gh project item-add $PROJECT_NUMBER --owner tzervas --url https://github.com/tzervas/embeddenator/issues/24
```

### 5. Set Initial Status

```bash
# Mark completed issues
gh project item-edit --project-id $PROJECT_NUMBER --id <item_id> --field-id <status_field_id> --value "Complete"
```

For issues #18, #19, #20, #21 (already closed).

---

## Current State (2026-01-04)

### Component Extraction Progress

| Component | Issue | Status | Column | Notes |
|-----------|-------|--------|--------|-------|
| embeddenator-vsa | #18 | ✅ Closed | Complete | v0.2.0 released |
| embeddenator-retrieval | #19 | ✅ Closed | Complete | v0.2.0 released |
| embeddenator-fs | #20 | ✅ Closed | Complete | v0.2.0 released |
| embeddenator-interop | #21 | ✅ Closed | Complete | v0.2.0 released |
| embeddenator-io | #22 | ⏳ Open | Backlog | Next - independent |
| embeddenator-obs | #23 | ⏹️ Open | Backlog | Ready - independent |
| **Epic Tracker** | #24 | 📊 Open | - | 66.7% complete |

### Branch Tracking

| Branch | Component | Status | Last Commit |
|--------|-----------|--------|-------------|
| feat/extract-vsa | embeddenator-vsa | ✅ Merged | Tagged v0.2.0 |
| feat/extract-retrieval | embeddenator-retrieval | ✅ Merged | Tagged v0.2.0 |
| feat/extract-fs | embeddenator-fs | ✅ Merged | Tagged v0.2.0 |
| feat/extract-interop | embeddenator-interop | ✅ Current | fe85433 - tracking update |
| feat/extract-io | embeddenator-io | ⏹️ Pending | Not yet created |
| feat/extract-obs | embeddenator-obs | ⏹️ Pending | Not yet created |

---

## Alternative: Manual GitHub Web UI Setup

If CLI authentication fails:

1. Go to https://github.com/users/tzervas/projects
2. Click "New project"
3. Choose "Board" template
4. Name: "Phase 2A: Component Extraction"
5. Add columns: Backlog, In Progress, Review, Complete
6. Link issues #18-#24 from embeddenator repo
7. Drag to appropriate columns based on current status

---

## Tracking Without Projects

If GitHub Projects setup is blocked, current tracking mechanisms are sufficient:

1. **SPLIT_TRACKER.md** - Markdown progress table in monorepo
2. **Issue #24** - Epic tracker with weekly updates
3. **Individual issues** (#18-#23) - Component-specific progress
4. **Git branches** - Feature branches track extraction work
5. **Git tags** - Component releases (v0.2.0 tags on each repo)

**Current Status Visible In:**
- Issue #24 body (updated 2026-01-04)
- SPLIT_TRACKER.md (66.7% complete)
- Closed issues (#18, #19, #20, #21)
- Component repo tags (all v0.2.0)

---

## Next Steps

1. **Complete Phase 2A** - Extract io + obs (2/6 remaining)
2. **GitHub Projects** - Set up when authentication available
3. **Documentation** - Maintain SPLIT_TRACKER.md as primary source

**Progress:** 4/6 components complete (66.7%), 8,664/9,564 LOC extracted (90.6%)  
**Timeline:** On track for Jan 28 completion (Week 2 of 4)
