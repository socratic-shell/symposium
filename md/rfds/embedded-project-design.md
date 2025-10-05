<!-- 

Instructions:

* Copy this file and give it a name like my-feature-slug.md
* Do not remove the `>` sections -- they are part of the template!
* Replace the HTML comments that give general guidance with your answers and content. Do not begin your answers with `>`, your text should be unquoted.
* For optional sections, you can leave the HTML comment as is if you do not wish to provide an answer.
* In the FAQ, begin each question with a markdown `##` section so that it gets a linkable anchor.

-->

# Elevator pitch

> What are you proposing to change?

Using Symposium today requires creating a "Symposium project" which is a distinct checkout and space from the user's ongoing work. This RFD lays out a plan to modify Symposium so that it can be used on an existing checkout and be added directly to the user's workflows.

# Status quo

> How do things work today and what problems does this cause? Why would we change things?

Today, using Symposium requires creating a separate "Symposium project" that is distinct from the user's existing work:

## Current Directory Structure

When users create a Symposium project, they get a structure like:
```
/path/to/symposium-projects/my-project/
├── .git/                    # Bare clone of their repository (a bit unusual)
├── .symposium/
│   ├── project.json         # Project configuration
│   └── task-$UUID/          # Individual taskspaces
│       ├── taskspace.json
│       └── my-project/      # Working copy for this taskspace
└── (other project files)
```

This creates a completely separate checkout from where the user normally works.

## Current Setup Process

To try Symposium, users must:

1. **Fill out a project creation form** with:
   - Project name
   - Git repository URL 
   - Local directory location
   - AI agent configuration
2. **Have a cloneable repository** - they can't experiment without existing git-hosted code
3. **Wait for git clone** - Symposium creates its own fresh checkout
4. **Context switch** - move from their normal working directory to the Symposium project directory
5. **Learn new concepts** - understand projects vs taskspaces before getting value

## Problems This Creates

**Workflow disruption**: Users must stop their current work and switch to a separate Symposium environment. This is disruptive and creates friction for adoption.

**Setup barriers**: The multi-step form and repository requirement prevent quick experimentation. Users can't just "try it on this code I'm looking at right now."

**Cognitive overhead**: Users must understand the Symposium project concept and directory structure before they can experience any value from AI collaboration.

**Maintenance burden**: Users end up with multiple checkouts of the same repository that can get out of sync or consume extra disk space.

# What we propose to do about it

> What are you proposing to improve the situation? 

Replace the "create Symposium project" workflow with an "open existing project" approach that works directly on the user's current checkout:

## New Project Opening Flow

1. **No splash screen needed** - users go directly to "Open Project"
2. **Select existing git directory** - point Symposium at where they're already working
3. **Automatic setup** - if no `.symposium` directory exists:
    - Prompt user that we'll create one
    - Modify `.gitignore` to exclude `.symposium`
        - Create a commit with *just* this one change and commit message "add symposium to gitignore" and "co-authored-by: socrates@symposium-dev.com"
        - If there are already staged changes, unstage and restage I guess? Or just don't commit it.
    - Create `.symposium` directory structure

## New Directory Structure

Instead of a separate checkout, Symposium works in-place:
```
/home/dev/my-project/                # User's existing project
├── .git/                           # Their existing git repository
├── .gitignore                      # Modified to include .symposium
├── .symposium/                     # Symposium metadata (gitignored)
│   ├── project.json                # project-wide configuration
│   ├── root-taskspace.json         # taskspace description for the "root", created by default
│   └── taskspace-$UUID/
│       ├── taskspace.json
│       └── my-project/             # Working copy for this taskspace
└── (user's existing project files)
```

## Root taskspace

Every project gets a default `root-taskspace.json` that works like any other taskspace but:
- Found at `.symposium/root-taskspace.json` instead of `.symposium/taskspace-$UUID/taskspace.json`
- Cannot be deleted (ensures users always have a working space)
- Provides immediate usability without requiring taskspace creation

This means the code must handle both lookup patterns gracefully and enforce the deletion restriction for the root taskspace.

## Other bits of auto-configuration

* We should auto-detect main branch
    * Look for a remote that is a non-fork github and see what it's default push target is
    * Failing that, present users with a choice

# Shiny future

> How will things will play out once this feature exists?

A developer working on their project decides to try Symposium:

1. **Opens Symposium** and selects "Open Project"
2. **Points to their current directory** - the one they're already working in
3. **Gets a simple prompt** - "We'll add Symposium support to this project, okay?"
4. **Root taskspace launches automatically** - opens with an agent that gets context about:
   - Current unstaged changes in the working directory
   - Recent commits that haven't been merged to main
   - Standard "find out what the user wants to do" prompt
5. **Immediately starts collaborating** - agent is aware of current work state
6. **Continues normal workflow** - their existing tools, git history, and working directory remain unchanged

# Implementation details and plan

> Tell me more about your implementation. What is your detailed implementaton plan? 

Let's begin migrating "business logic" out from the Swift code and into the Rust code to make it more portable. Let's extend the `symposium-mcp` command to have a new command, `private` -- ideally, undocumented. We can then add commands like

```bash
symposium-mcp private open-symposium-project --path "..."
```

which will do the work of initializing the directory and respond with a JSON structure.

# Frequently asked questions

> What questions have arisen over the course of authoring this document or during subsequent discussions? 


# Questions for discussion

## Why do we create two subdirectories?

The reason we create this structure:

* `taskspace-$UUID/my-project`

rather than just `taskspace-$UUID` is that it means that, within VSCode, the project appears as `my-project` and not a UUID.