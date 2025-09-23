# Requests for Dialog (RFDs)

{RFD:introduce-rfd-process}

A "Request for Dialog" (RFD) is Symposium's version of the RFC process. RFDs are the primary mechanism for proposing new features, collecting community input on an issue, and documenting design decisions.

## When to write an RFD

You should consider writing an RFD if you intend to make a "substantial" change to Symposium or its documentation. What constitutes a "substantial" change is evolving based on community norms and varies depending on what part of the ecosystem you are proposing to change.

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

RFDs are living documents that track implementation progress. PRs working towards an RFC will typically update it to reflect changes in design or direction.

When adding new content into the mdbook's design section that is specific to an RFD, those contents are marked with RFD badges, written e.g. `{RFD:rfd-name}`. An mdbook preprocessor detects these entries and converts them into a proper badge based on the RFD's status.

### 2b. Move to "To be removed"

RFDs that have never landed may be closed at the discretion of a core team member. RFDs that have landed in draft form are moved to "To be removed" instead until there has been time to remove them fully from the codebase, then they are removed entirely.

### 3. Move to "Preview" when fully implemented

When the champion feels the RFD is ready for broader review, they open a PR to move it to "Preview." This signals the community to provide feedback. The PR stays open for a few days before the champion decides whether to land it.

### 4. Completed

Once in preview, the RFD can be moved to "completed" with a final PR. The core team should comment and express concerns, but **final decision is always made by the core team lead**. Depending on what the RFD is about, "completed" is the only state that can represent a 1-way door (if there is a stability commitment involved), though given the nature of this project, many decisions can be revisited without breaking running code.

Preview RFDs don't have to be completed. They may also go back to draft to await further changes or even be moved ot "To be removed".

### 5. Implementation and completion

## RFD Lifecycle

- **Early drafts**: Initial ideas, brainstorming, early exploration
- **Mature drafts**: Well-formed proposals ready for broader review  
- **Accepted**: Approved for implementation, may reference implementation work
- **To be removed (yet?)**: Decided against for now, but preserved for future consideration
- **Completed**: Implementation finished and merged

## Governance

The project has a design team with nikomatsakis as the lead (BDFL). Champions from the core team guide RFDs through the process, but final decisions rest with the team lead. This structure maintains velocity while anticipating future governance expansion.

## Discussion and Moderation

Detailed discussions happen on Zulip, with PR comments for process decisions. RFD champions actively curate discussions by collecting questions in the FAQ section. If PR discussions become too long, they should be closed, feedback summarized, and reopened with links to the original.

## Licensing

All RFDs are dual-licensed under MIT and Apache 2.0. The project remains open source, with the core team retaining discretion to change to other OSI-approved licenses if needed.
