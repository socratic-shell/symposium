# Elevator pitch

> Introduce a "Request for Dialog" (RFD) process to replace ad-hoc design discussions with structured, community-friendly design documents that track features from conception to completion.

# Status quo

> Currently we have design documents scattered across different sections, work-in-progress tracking that's disconnected from design rationale, and no clear process for community members to propose or discuss new features. When we open source, people won't know how to contribute design ideas or understand the reasoning behind decisions.

# Shiny future

> Community members can propose RFDs for new features, existing RFDs document the reasoning behind current work, and status badges in design docs link directly to the relevant RFDs. Agents can read RFDs to understand not just how things work, but why they were designed that way.

# Implementation plan

> ✅ Create RFD infrastructure (README, template, SUMMARY.md sections)
> ✅ Establish lifecycle: Early drafts → Mature drafts → Accepted → Completed  
> ⏳ Connect status badges to RFDs
> ⏳ Write RFDs for major in-progress features
> ⏳ Document community contribution process

# Frequently asked questions

## What alternative approaches did you consider, and why did you settle on this one?

We considered using GitHub issues for design discussions, but RFDs living in-repo means agents can reference them during implementation. We also considered keeping the formal RFC process, but "Request for Dialog" better captures the collaborative, exploratory nature we want.

## Why "Request for Dialog" instead of "Request for Comments"?

"Dialog" emphasizes conversation and exploration rather than just collecting feedback on a predetermined design. It fits better with our collaborative development philosophy.

# Revision history

- 2025-09-17: Initial version, created alongside RFD infrastructure
