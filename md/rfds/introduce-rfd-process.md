# Elevator pitch

> What are you proposing to change? Bullet points welcome.

Introduce a "Request for Dialog" (RFD) process to replace ad-hoc design discussions with structured, community-friendly design documents that track features from conception to completion.

# Status quo

> How do things work today and what problems does this cause? Why would we change things?

Currently all development is being done by nikomatsakis and tracking documents are scattered all over the place. The goal is to create a process that helps to keep files organized and which can scale to participation by an emerging community.

# Shiny future

> How will things will play out once this feature exists?

## Proposing "early drafts"

Community members fork the repo and add a new RFD file to the "Early drafts" section using kebab-case naming. The RFD can start minimal - just an elevator pitch and status quo are enough to begin dialog. Pull requests become the discussion forum where ideas get refined through collaborative iteration.

## Highlighting "mature drafts"

When an RFD has been fleshed out with clear problem definition, proposed solution, and implementation approach, team members move it to "Mature drafts." This signals it's ready for broader review and final decision-making. Mature drafts get highlighted in project updates to ensure visibility.

## Deciding whether to accept an RFD or not

The team reviews mature drafts during regular planning cycles. Accepted RFDs move to the "Accepted" section and become candidates for implementation. RFDs can also move to "Not accepted (yet?)" if they're good ideas but not the right priority - preserving the thinking for future reconsideration.

## Implementation of an RFD

Once accepted, RFDs become living documents that track implementation progress. Status badges in design documentation link back to the relevant RFD, creating a clear connection between "why we're building this" and "how it works." Agents can read RFDs during implementation to understand design rationale and make consistent decisions.

# Implementation plan

> What is your implementaton plan?

* ✅ Create RFD infrastructure (README, template, SUMMARY.md sections)
* ✅ Establish lifecycle: Early drafts → Mature drafts → Accepted → Completed  
* ⏳ Connect status badges to RFDs
* ⏳ Write RFDs for major in-progress features
* ⏳ Document community contribution process

# Frequently asked questions

## Why "Request for Dialog" and not "Request for Comment"?

Well, partly as a nod to socratic dialogs, of course, but also because "dialog" emphasizes conversation and exploration rather than just collecting feedback on a predetermined design. It fits better with our collaborative development philosophy.

## Why make changes to Rust's RFC template?

These changes are based on nikomatsakis's experience with Rust RFCs and are an attempt to better focus user's on what nikomatsakis considers "Best practice" when authoring RFCs.

## Why are you tracking the implementation plan in the RFD itself?

We considered using GitHub issues for design discussions, but RFDs living in-repo means agents can reference them during implementation.

## Why not have RFD numbers?

It's always annoying to keep those things up-to-date and hard-to-remember. The slug-based filenames will let us keep links alive as we move RFDs between phases.

# Revision history

- 2025-09-17: Initial version, created alongside RFD infrastructure
