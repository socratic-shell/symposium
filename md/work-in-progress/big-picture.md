# Big picture plans and progress

## Phases

* ✅ *Done: proof-of-concept for synchronous execution*
* ① *Phase 1: proof-of-concept for asynchronous execution on localhost*
* ② *Phase 2: expand to more editors, more agents, more configuration options*
* ⏳ *Planned for the future but not on a concrete timeline*

## App startup experience

*On first startup, app configures everything*

* ✅ Detects required permissions
* ⏳ Installs extensions for editors you plan to use

## Editor choices

*Support a wide variety of editors within a taskspace*

* ✅ VSCode
* ② Terminal
* ② IntelliJ and RustRover
* ⏳ Vim.app (does this exist?)
* ⏳ Emacs.app

## Agent choices

*Support a wide variety of AI agents*

* ✅ CLI agents -- these can be used for autonom
    * ✅ Claude Code
    * ✅ Q CLI
    * ⏳ Aider
* ⏳ Non-CLI agents like Cline, Kiro, and friends
    * We do want to support these, but unclear how to support them outside of a localhost, non-containerized environment

## "Attached" execution (non-containerized)

*User can work on localhost in maximally compatible fashion with existing workflows*

* ✅ Synchronous agent that runs only when connected
* ✅ Creates directory on localhost
* ✅ Clones project
* ✅ Starts editor
* ✅ Agent choices
    * ✅ Verifies CLI agent configuration on localhost
    * ⏳ Configures CLI agents for you
    * ⏳ Supports non-CLI agents

## "Detached" execution (containerized)

*User can work with 'detached agents' that run asynchronously on a variety of hosting platforms*

Core goals:

* ① Launching a new taskspace creates a new active taskspace that runs even when disconnected:
    * ① Localhost (probably a useful stepping stone)
    * ② SSH remote (good first goal)
    * ② Public ephemeral environments like Github codespaces
    * ② Private ephemeral environments configured via plugin
* ① Agent runs asynchronously
    * Launch the CLI tool in a process in that taskspace connected via tmux or some other thing
* ① Project repository includes 'development configuration'
    * ① Dockerfile
    * ② DevContainer
    * App automatically extends that configuration with
        * ① Pre-configured CLI agent of choice (Claude Code, Q CLI, etc)
        * ① Our MCP server
* ① When "disconnected":
    * ① App should attempt connect with the CLI agent even when editor is not visible
    * ① Permits logging etc
* ① When "connected"
    * ① App should open editor of choice and let user work with project files + agent
    * ① App should make the CLI agent visible in the editor
    * ② App should make the CLI agent visible in a separate terminal window that it controls

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
