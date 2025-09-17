# Requests for Dialog (RFDs)

A "Request for Dialog" (RFD) is Socratic Shell's version of the RFC process. RFDs are the primary mechanism for proposing new features, collecting community input on an issue, and documenting design decisions.

## When to write an RFD

You should consider writing an RFD if you intend to make a "substantial" change to Socratic Shell or its documentation. What constitutes a "substantial" change is evolving based on community norms and varies depending on what part of the ecosystem you are proposing to change.

Some changes do not require an RFD:
- Rephrasing, reorganizing or refactoring
- Addition or removal of warnings
- Additions that strictly improve objective, numerical quality criteria (speedup, better browser support)
- Fixing objectively incorrect behavior

## The RFD Process

### 1. Propose by opening a PR

Fork the repo and copy `rfds/TEMPLATE.md` to `rfds/my-feature.md` (using kebab-case naming). The RFD can start minimal - just an elevator pitch and status quo are enough to begin dialog. Pull requests become the discussion forum where ideas get refined through collaborative iteration.

### 2. Merge to "Draft" when championed

RFD proposals are merged into the "Draft" section if a core team member decides to champion them. The champion becomes the point-of-contact and will work with authors to make it reality. Once in draft, implementation may begin (properly feature-gated with the RFD name).

### 3. Move to "Preview" when ready

When the champion feels the RFD is ready for broader review, they open a PR to move it to "Preview." This signals the community to provide feedback. The PR stays open for a few days before the champion decides whether to land it.

### 4. Accept or reject

When ready for completion, the champion requests team review. Final decision is made by the core team lead. Accepted RFDs move to "Accepted" section, rejected ones go to "Not accepted (yet?)" for future consideration.

### 5. Implementation and completion

Accepted RFDs become living documents tracking implementation progress. Status badges in design docs link back to RFDs. Agents read and update RFDs during implementation to maintain design rationale.

**Badge Convention:** When implementing RFD features, use WIP badges in design documentation:
```markdown
# Window Stacking System [![WIP: window-stacking](https://img.shields.io/badge/WIP-window--stacking-yellow)](../rfds/window-stacking.md)
```

This creates a direct connection between implementation documentation and the RFD tracking progress, design decisions, and open questions.

## RFD Lifecycle

- **Early drafts**: Initial ideas, brainstorming, early exploration
- **Mature drafts**: Well-formed proposals ready for broader review  
- **Accepted**: Approved for implementation, may reference implementation work
- **Not accepted (yet?)**: Decided against for now, but preserved for future consideration
- **Completed**: Implementation finished and merged

## Governance

The project has a design team with nikomatsakis as the lead (BDFL). Champions from the core team guide RFDs through the process, but final decisions rest with the team lead. This structure maintains velocity while anticipating future governance expansion.

## Discussion and Moderation

Detailed discussions happen on Zulip, with PR comments for process decisions. RFD champions actively curate discussions by collecting questions in the FAQ section. If PR discussions become too long, they should be closed, feedback summarized, and reopened with links to the original.

## Licensing

All RFDs are dual-licensed under MIT and Apache 2.0. The project remains open source, with the core team retaining discretion to change to other OSI-approved licenses if needed.
