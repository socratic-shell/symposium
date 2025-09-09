# Big picture plans and progress

## Major planned workstreams

* ✅ *Done: proof-of-concept for synchronous execution*
* ① *Worktrees*
* ② *Remote hosting via ssh*
* ③ *Persistent, asynchronous agents*
* ④ *Misc UI improvements*
* ⑤ *Socratic Shell improvements*
* ⑥ *Multiwindow taskspaces*
* ⏳ *No specific plan*

## App startup experience

*On first startup, app configures everything*

* ✅ Detects required permissions
* ⏳ Editor choices
    * ⏳ Verifies available editors
    * ⏳ Installs extensions for editors you plan to use
* ✅ Agent choices
    * ✅ Verifies CLI agent configuration on localhost
    * ⏳ Configures CLI agents for you
    * ⏳ Supports non-CLI agents

## Creating a project

* User selects where to host the project
    * ✅ Localhost
    * ② SSH remote -- supporting will require cross-cutting work to execute via ssh
* ✅ Creating new projects
    * ✅ Creates base project directory structure

## Opening and viewing a project

* ✅ Taskspace window appears showing existing taskspaces with previous screenshots
* ✅ User can expand a taskspace to view logs
* ④ User can rearrange taskspace ordering at will
* ③ When agent sends taskspace updates (note that taskspace need not be connected)
    * ③ Logs are recorded and updated
    * ④ When a taskspace has requested user attention, dock icon is badged, badge is cleared when user activates taskspace

## Creating a taskspace

* ✅ Creating new taskspace with UUID `$U`
    * ① Create the "master" `.git` if needed
    * ① Fresh worktree in branch `task-$U`
    * ③ Spawn autonomous agent operating in that worktree (record the PID)
* ✅ Initial prompt guides taskspace to setup:
    * ✅ a name
    * ✅ a description
    * ① a branch name

## Selecting a taskspace

* ✅ Selecting existing taskspace with UUID `$U`
    * ✅ Attaches editor if needed, activates editor window to the front otherwise
    * ③ Connect to the agent

## Development experience

*Provide agent with tools beyond awk/grep to make coding more effective*

* ✅ Fetch examples of how to use libraries
* ⏳ Upgrade guidance -- how to move across versions
* ✅ Structured access to IDE services
    * ✅ Find definition and find references
    * ⏳ Rename symbol
    * ⏳ Hover and type hints
* ⏳ Best practice guidance

We wish to support many languages, starting with:

* ✅ Rust
* ⏳ Python
* ⏳ TypeScript
* ⏳ Swift
* ...and everything else

## Telemetry

*When opted in, reports telemetry in a differential, privacy-preserving fashion*

* ⏳ Locally analyzes transcripts to judge
    * Agent collaborativeness
    * Agent effectiveness

## Enhanced collaboration style

*Context to make AI agent learn and work better with user as a partner*

* ✅ Core context
* ⏳ Remembers and records facts about
    * current task (specific to current task)
    * current project (applicable to any task in the same project)
    * collaborative patterns

## Team member collaboration

* ⏳ Shares memories 

## Taskspace coordination

* ⏳ Shares results and data across taskspaces

## Configuration options

### Editor choices

*Support a wide variety of editors within a taskspace*

* ✅ VSCode
* ② Terminal
* ② IntelliJ and RustRover
* ⏳ Vim.app (does this exist?)
* ⏳ Emacs.app

### Window choices

* Agent can be
    * ✅ in editor terminal
    * ⑥ in separate terminal
    * ⏳ IDE-integrated
* ⏳ Users can configure and attach other windows to the taskspace

### Agent choices

*Support a wide variety of AI agents*

* ✅ CLI agents -- these can be used for autonom
    * ✅ Claude Code
    * ✅ Q CLI
    * ⏳ Aider
* ⏳ Non-CLI agents like Cline, Kiro, and friends
    * We do want to support these, but unclear how to support them outside of a localhost, non-containerized environment

### Execution model

* ✅ "Direct" execution model
    * Each taskspace is a directory, agents are processes; maximally compatible fashion with existing workflows
    * Can run on:
        * ✅ localhost
        * ② ssh remotes
    * User is responsible for:
        * setting up their dev tools and environment
        * installing CLI agents on the desired host
* "Containerized" execution (containerized)
    * TBD -- containerized execution will be used for ephemeral compute but requires resolving auth and setup issues (e.g., Claude Code does not allow you to easily setup a container with auth tokens).