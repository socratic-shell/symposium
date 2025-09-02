# Interface plan

## On startup, display settings

Check whether the app has been granted Accessibility preferences and whether we have recorded User Preferences. If either is false go open the settings dialog.

## Settings dialog

The settings dialog displays:

* Accessibility permissions
    * Document: "Used to move windows to the front and resize them for tiling. Required."
    * Either a green check and "Granted"
    * Or a red "X" and a button "Request"
        * it should say "Symposium requires accessibility permissions."
    * Include a "debug" option that will run `tccutil reset Accessibility com.symposium.app` to clear out the settings.
* Screen recording permissions
    * Document: "Used to create snapshots, optional"
    * Either show a green or a yellow warning sign.
* And it should have some preferences like
    * On connecting to a new taskspace start: (each applicaiton has a checkbox)
        * IDEs: (system scans for installed IDEs and shows only available ones)
            * [ ] VSCode.app - Status: Extension installed ✓ / Extension missing ✗ (Install button)
            * [ ] IntelliJ.app (planned for later) 
            * [ ] Emacs.app in a window (planned for later)
            * [ ] Emacs.app in a terminal (planned for later)
            * [ ] Vi in a Terminal (planned for later)
    * Later on we will add more options
        * Terminal
        * URLs
            * [ ] (let users select the URLs to add)
        * Other apps:
            * [ ] Safari (if it is found on the user's system)
            * [ ] Firefox (if it is found on the user's system; also look for DEveloper edition etc)
            * [ ] Chrome (if it is found on the user's system)
            * [ ] Other (let's users click and add others)
    * How do you want to communicate with the agent:
        * Inside the IDE (uses VSCode integrated terminal with extension)
        * Separate terminal (extension installs agent but runs in separate terminal window)
    * Agent tool selection:
        * [ ] Claude Code
        * [ ] Q CLI

## "Taskspace" concept

An "taskspace" is the combination of

* an active AI agent
* one or more developer windows, such as
    * an IDE like vscode or intellij
    * a terminal
    * a browser
* a local workspace directory (UUID-named for isolation)

### MVP Taskspace Workflow

For the MVP, each taskspace consists of:
* One VSCode window opened to a local directory
* VSCode's integrated terminal running an AI agent (Claude Code or Q CLI)
* The agent is automatically started with an initial prompt when the taskspace is created

### Taskspace Creation

Taskspaces are created when an AI agent invokes the MCP server's `spawn_taskspace` tool with:
* `name` - suitable name for the taskspace
* `task_description` - description of work to be done
* `initial_prompt` - prompt provided to the new agent when it starts

The system then:
1. Creates a local directory with UUID name for workspace isolation
2. Launches VSCode with that directory as the workspace
3. The pre-installed VSCode extension automatically opens the integrated terminal
4. The extension launches the configured agent tool (Claude Code or Q CLI) with the initial prompt
5. The background agent begins working autonomously on the specified task

### Agent Coordination

Background agents can coordinate with Symposium via MCP tools:
* `log_progress(message, category)` - Report progress with visual indicators (ℹ️ info, ⚠️ warn, ❌ error, ✅ milestone, ❓ question)
* `signal_user(message)` - Request user attention, causing the taskspace to move toward the front of the panel

## Dock badging

The Dock icon updates itself to reflect the number of active taskspaces along with the number of that are requesting attention.

## The Symposium panel

Symposium's main interface is an overview panel that display an overviews of each active taskspace along with buttons to configure the layout of the windows associated with the agent space:
```
                                                          
 ┌─────────────────────────────────────────────────────┐ 
 │ ┌────────┐  ┌───────┐  ┌───┬───┐  ┌───┬───┐         │ 
 │ │        │  │       │  │   │   │  │   │   │         │ 
 │ │ Logo   │  ┼───────┼  │   │   │  ┼───┴───┼ ...     │ 
 │ │        │  │       │  │   │   │  │       │         │ 
 │ └────────┘  └───────┘  └───┴───┘  └───────┘         │ 
 │                                                     │ 
 │  ────────────────────────────────────────────────   │ 
 │                                                     │ 
 │ ┌──────────────────────┐                            │ 
 │ │                      │ *Agent space name*         │ 
 │ │ ┌───────┐   ┌────┐   │                            │ 
 │ │ │       │   │    │   │ * Log 1                    │ 
 │ │ │       │   │    │   │ * Log 2                    │ 
 │ │ │       │   └────┘   │ * ..                       │ 
 │ │ │       │            │ * Log N                    │ 
 │ │ │       │ ┌────────┐ │                            │ 
 │ │ └───────┘ │        │ │                            │ 
 │ │           │        │ │                            │ 
 │ │           └────────┘ │                            │ 
 │ └──────────────────────┘                            │ 
 │                                                     │ 
 │  ─────────────────────────────────────────────────  │ 
 │                                                     │ 
 │ ┌──────────────────────┐                            │ 
 │ │                      │ *Another agent space*      │ 
 │ │ ┌──────────────────┐ │                            │ 
 │ │ │                  │ │ * Log 1                    │ 
 │ │ │                  │ │ * ...                      │ 
 │ │ │                  │ │                            │ 
 │ │ │                  │ │                            │ 
 │ │ └──────────────────┘ │                            │ 
 │ │ ┌──────────────────┐ │                            │ 
 │ │ │                  │ │                            │ 
 │ │ └──────────────────┘ │                            │ 
 │ └──────────────────────┘                            │ 
 │                                                     │ 
 └─────────────────────────────────────────────────────┘ 
                                                         
```

The panel is divided into sections with:

* The top section showing various tiling configurations
    * Clicking these buttons will position the windows in the shown configuration, maximizing them over the display but with space left for the panel
    * When they are tiled in this way, we monitor for resize events
    * When windows are resized, we resize the others to match
    * If windows are moved they return to "free form mode" where their position is not actively monitored by the app
* The other sections representing active taskspaces, with the current taskspace at the top
* Each taskspace contains
    * Periodic screenshots of the windows associated with the Taskspace, positioned in the same way relative to the overall display
    * The name of the taskspace  
    * A display of logs generated by the agent via `log_progress` MCP tool calls, showing progress messages with visual indicators
* When agents use `signal_user` to request assistance, their taskspace is automatically ordered closer to the front of the panel

## Panel behavior

For the MVP, the panel appears only when the Dock icon is clicked. It appears in a speech bubble similar to how a Folder's display their contents when placed into the dock.

Later on we will add options to anchor the panel to the left/right of the screen and so forth.

## Out of scope for now

We are focused on interacting with agents through CLI tools. We aim to be agnostic with respect to the specific tool and to allow others to easily add new kinds of CLI tools. Some IDEs (Cursor, Kiro, Windsurf) have embedded agents and things.  We will ignore those for now but the intention is eventually to support them.