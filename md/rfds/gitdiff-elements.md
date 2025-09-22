# Elevator pitch

Add `gitdiff` elements to the walkthrough system to display interactive git diffs within walkthroughs, allowing agents to show code changes inline with explanatory content.

# Status quo

Currently, when agents want to show code changes in walkthroughs, they have limited options:
- Reference code with comments, but can't show what changed
- Describe changes in text, which is less clear than visual diffs
- Ask users to manually check git history to understand changes

This makes it harder to create comprehensive walkthroughs that explain both the current state of code and how it evolved. When demonstrating development workflows or explaining implementation decisions, the lack of inline diff visualization breaks the narrative flow.

# What we propose to do about it

Implement `gitdiff` elements in the walkthrough markdown format that render as interactive diff trees in VSCode. The syntax would be:

```markdown
```gitdiff(range="HEAD~2..HEAD")
```

```markdown
```gitdiff(range="abc123", exclude-unstaged, exclude-staged)
```

This would allow agents to seamlessly integrate git diffs into educational walkthroughs, showing exactly what code changed while explaining the reasoning behind those changes.

# Shiny future

Agents will be able to create rich, educational walkthroughs that combine:
- Explanatory text and mermaid diagrams
- Interactive code comments
- Visual git diffs showing actual changes
- Seamless narrative flow from "here's what we built" to "here's how we built it"

This will make Symposium's learning capabilities much more powerful, especially for onboarding to new codebases or understanding complex changes.

# Frequently asked questions

## What alternative approaches did you consider, and why did you settle on this one?

We considered static code blocks with diff syntax highlighting, but interactive diffs provide much better user experience. We also considered linking to external git hosting, but keeping everything in the walkthrough maintains the narrative flow.

# Revision history

- 2025-09-22: Initial draft
