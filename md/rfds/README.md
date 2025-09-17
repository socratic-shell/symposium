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

In short, to get a major feature added to Socratic Shell, one usually first gets the RFD merged into the RFD repository as a markdown file. At that point the RFD is "active" and may be implemented with the goal of eventual inclusion into Socratic Shell.

1. **Early Draft**: Fork the repo and copy `rfds/TEMPLATE.md` to `rfds/my-feature.md` (where "my-feature" is descriptive). Fill in the RFD. Put care into the details: RFDs that do not present convincing motivation, demonstrate understanding of the impact of the design, or are disingenuous about the drawbacks or alternatives tend to be poorly-received.

2. **Mature Draft**: Submit a pull request. As a pull request the RFD will receive design feedback from the larger community, and the author should be prepared to revise it in response.

3. **Accepted**: Eventually, the team will decide whether the RFD is a candidate for inclusion in Socratic Shell. RFDs that are candidates for inclusion in Socratic Shell will enter a "final comment period" lasting 3 calendar days. At the end of this period, the team will either accept or reject the RFD.

4. **Implementation**: Once an RFD becomes active, then authors may implement it and submit the feature as a pull request to the Socratic Shell repo. Becoming "active" is not a rubber stamp, and in particular still does not mean the feature will ultimately be merged; it does mean that the core team has agreed to it in principle and are amenable to merging it.

5. **Completed**: Furthermore, the fact that a given RFD has been accepted and is "active" implies nothing about what priority is assigned to its implementation, nor whether anybody is currently working on it.

## RFD Lifecycle

- **Early drafts**: Initial ideas, brainstorming, early exploration
- **Mature drafts**: Well-formed proposals ready for broader review
- **Accepted**: Approved for implementation, may reference implementation work
- **Not accepted (yet?)**: Decided against for now, but preserved for future consideration
- **Completed**: Implementation finished and merged

## Reviewing RFDs

Each week the team will attempt to review some set of open RFD pull requests.

We try to make sure that any RFD that we accept is accepted for the right reasons, and because we have a good understanding of what it means to support that feature long term.

## Implementing an RFD

The author of an RFD is not obligated to implement it. Of course, the RFD author (like any other developer) is welcome to post an implementation for review after the RFD has been accepted.

If you are interested in working on the implementation for an "active" RFD, but cannot determine if someone else is already working on it, feel free to ask (e.g. by leaving a comment on the associated issue).
