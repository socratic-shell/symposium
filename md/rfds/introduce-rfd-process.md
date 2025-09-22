# Elevator pitch

> What are you proposing to change? Bullet points welcome.

Introduce a "Request for Dialog" (RFD) process to replace ad-hoc design discussions with structured, community-friendly design documents that track features from conception to completion.

# Status quo

> How do things work today and what problems does this cause? Why would we change things?

Currently all development is being done by nikomatsakis and tracking documents are scattered all over the place. The goal is to create a process that helps to keep files organized and which can scale to participation by an emerging community.

# Shiny future

> How will things will play out once this feature exists?

## Project licensing

All code and RFDs are dual-licensed under a MIT and Apache 2.0 license. The project is intended to remain open source and freely available in perpetuity. By agreeing to the MIT licensing, contributors agree that the core team may opt to change this licensing to another [OSI-approved open-source license](https://opensource.org/licenses) at their discretion in the future should the need arise.

## Decision making

For the time being, the project shall have a "design team" with 1 member, nikomatsakis, acting in "BDFL" capacity. The expectation is that the project will setup a more structure governance structure as it grows. The design team makes all decisions regarding RFDs and sets overall project direction.

## RFD lifecycle

### RFDs are proposed by opening a PR

An RFD begins as a PR adding a new file into the "Draft" section. The RFD can start minimal - just an elevator pitch and status quo are enough to begin dialog. Pull requests become the discussion forum where ideas get refined through collaborative iteration. 

As discussion proceeds, the FAQ of the RFD should be extended. If discussion has been going long enough, the PR should be closed, feedback summarized, and then re-opened with a link to the original PR.

### The PR is merged into "draft" once a core team member decides to champion it

RFD proposals are merged into the "draft" section if a core team member decides to champion them. The champion is then the point-of-contact for that proposal going forward and they will work with the proposal authors and others to make it reality. Core team members do not need to seek consensus to merge a proposal into the draft, but they should listen carefully to concerns from other core team members, as it will be difficult to move the RFD forward if those concerns are not ultimately addressed.

Once a proposal is moved to draft, code and implementation may begin to land into the PR. This work needs to be properly feature gated and marked with the name of the RFD.

Further discussion on the RFD can take place on Zulip.

### Moving to the "preview" section

Once the champion feels the RFD is ready for others to check it out, they can open a PR to move the file to the preview section. This is a signal to the community (and particularly other core team members) to check out the proposal and see what they think. The PR should stay open for "a few days" to give people an opportunity to leave feedback. The champion is empowered to decide whether to land the PR. As ever, all new feedback should be recorded in the FAQ section.

### Deciding to accept an RFD

When they feel the RFD is ready to be completed, the champion requests review by the team. The team can raise concerns and notes during discussion. Final decision on an RFD is made by the core team lead. 

### Implementation of an RFD

Once accepted, RFDs become living documents that track implementation progress. Status badges in design documentation link back to the relevant RFD, creating a clear connection between "why we're building this" and "how it works." When building code with an agent, agents should read RFDs during implementation to understand design rationale and update them with implementation progress.

## Moderating and managing RFD discussions

Moving RFDs between points in the cycle involve opening PRs. Those PRs will be places to hold people dialog and discussion -- but not the only place, we expect more detailed discussions to take place on Zulip. RFD owners and champions should actively "curate" discussions by collecting questions that come up and ensuring they are covered in the FAQ. Duplicate questions can be directed to the FAQ.

If the discussion on the PR gets to the point where Github begins to hide comments, the PR should typically be closed, feedback collected, and then re-opened.

# Implementation plan

> What is your implementaton plan?

* ✅ Create RFD infrastructure (README, template, SUMMARY.md sections)
* ✅ Establish lifecycle: Draft → Preview → Accepted → Completed  
* ⏳ Connect status badges to RFDs
* ⏳ Write RFDs for major in-progress features
* ⏳ Document community contribution process

# Frequently asked questions

## So...there's a BDFL?

Yes. Early in a project, a BDFL is a good fit to maintain velocity and to have a clear direction.

## So...why does this talk about a core team?

I am anticipating that we'll quickly want to have multiple designers who can move designs forward and not funnel *everything* through the "BDFL". Therefore, I created the idea of a "design team" and made the "BDFL" the lead of that team. But I've intentionally kept the *final* decision about RFDs falling under the lead, which means fundamentally the character of Symposium remains under the lead's purview and influence. I expect that if the project is a success we will extend the governance structure in time but I didn't want to do that in advance.

## Why "Request for Dialog" and not "Request for Comment"?

Well, partly as a nod to Socratic dialogs, but also because "dialog" emphasizes conversation and exploration rather than just collecting feedback on a predetermined design. It fits better with our collaborative development philosophy.

## Why not use Rust's RFC template as is?

We made some changes to better focus users on what nikomatsakis considers "Best practice" when authoring RFCs. Also because the new names seem silly and fun and we like that.

## Why are you tracking the implementation plan in the RFD itself?

We considered using GitHub issues for design discussions, but RFDs living in-repo means agents can reference them during implementation.

## Why not have RFD numbers?

It's always annoying to keep those things up-to-date and hard-to-remember. The slug-based filenames will let us keep links alive as we move RFDs between phases.

# Revision history

- 2025-09-17: Initial version, created alongside RFD infrastructure
