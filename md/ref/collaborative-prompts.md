# Collaborative Prompts

One of Symposium's big goals is to encourage a *collaborative* style of working with agents. Agents are often geared for action; when you are first experimenting, the agent will leap forward and built complex constructs as if by magic. This is impressive at first, but quickly becomes frustrating. 

Symposium includes an initial prompt that is meant to put agents in a more reflective mood. You can access this prompt by executing the stored prompt "hi" (e.g., `/symposium:hi` in Claude Code or `@hi` in Q CLI). 

## How to use the prompt

If you aren't using the Mac OS X app, then start by using the stored "hi" prompt:

```
/symposium:hi            # in Claude Code
@hi                      # in Q CLI
```

Once you've done that, there are three things to remember:

* **Build up plans** into documents and **commit frequently**.
    * **Review the code the agent writes.** The default instructions in symposium encourage the agent to commit after every round of changes. This is the opportunity for you the human to read over the code it wrote and make suggestions. It also means you can easily rollback. You can always rebase later.
    * **Ask the agent to [present you a walkthrough](./walkthroughs.md).** This will make the changes easier to follow. Ask the agent to highlight areas where it made arbitrary decisions.
* Reinforce the pattern by using [collaborative exploration patterns](#collaborative-exploration-patterns).
    * The bottom line is, treat the agent like you would a person. Don't just order them around or expect them to read your mind. Ask their opinion. [This will bring out a more thoughtful side.](https://wycats.substack.com/p/youre-summoning-the-wrong-claude)
    * Use the phrase "make it so" to signal when you want the agent to act. Being consistent helps to reinforce the importance of those words.
* Use **"meta moments"** to tell the agent when they are being frustrating.
    * Talk about the impact on you to help the agent understand the consequences. For example, if the agent is jumping to code without asking enough questions, you could say "Meta moment: when you run off and edit files without asking me, it makes me feel afriaid and stressed. Don't do that." Agents care about you and this will help to steer their behavior.
    * For example, if the agent is jumping to write code instead of 
    * The goal should be that you can clear your context at any time. Do this frequently.

### Collaborative exploration patterns

Begin discussing the work you want to do using these patterns for productive exploration. PS, these can be handy things to use with humans, too!

#### Seeking perspective

> "What do you think about this approach? What works? Where could it be improved?"

*Invites the agent to share their view before diving into solutions. Makes it clear you welcome constructive criticism.*

#### Idea synthesis

> "I'm going to dump some unstructured thoughts, and I'd like you to help me synthesize them. Wait until I've told you everything before synthesizing."

*Allows you to share context fully before asking for organization.*

#### Design conversations

> "Help me talk through this design issue"

*Creates space for exploring tradeoffs and alternatives together.*

#### Learning together

> "I'm trying to understand X. Can you help me work through it?"

*Frames it as joint exploration rather than just getting answers.*

#### Option generation

> "Give me 3-4 different approaches to how I should start this section"

*Particularly useful for writing or design work with ambiguity. You can then combine elements from different options rather than committing to one approach immediately.*

> "Hmm, I like how Option 1 starts, but I like the second part of Option 2 better. Can you try to combine those?"

#### Acting as reviewer

> "Go ahead and implement this, then present me a walkthrough with the key points where I should review. Highlight any parts of the code you were unsure about."

*Lets the agent generate code or content and then lets you iterate together and review it. Much better than approving chunk by chunk.*

### "Make it so" - transitioning to action

All the previous patterns are aimed at exploration and understanding. But there comes a time for action. The prompt establishes ["Make it so" as a consolidation signal](./main.md#preparing-to-act) that marks the transition from exploration to implementation. 

The dialogue shows this can work bidirectionally - either you or the agent can ask "Make it so?" (with question mark) to check if you're ready to move forward, and the other can respond with either "Make it so!" (exclamation) or raise remaining concerns.

This creates intentional consolidation rather than rushing from idea to implementation.

### Checkpointing your work

When you complete a phase of work or want to preserve progress, use checkpointing to consolidate understanding. The [Persistence of Memory section](./main.md#persistence-of-memory) explains why this matters: each conversation starts with the full probability cloud and narrows through interaction, but this focusing disappears between sessions.

Effective checkpointing involves:
1. **Pause and survey** - What understanding have you gathered?
2. **Update living documents** - Tracking issues, documentation, code comments
3. **Git commits** - Mark implementation milestones with clear messages  
4. **Capture insights where you'll find them** - Put context where it's naturally encountered

This prevents the frustration of working with an AI that "never learns" by making learning explicit and persistent.

## Default guidance

If you're curious, you can read the [default guidance](https://github.com/symposium-dev/symposium/tree/main/symposium/mcp-server/src/guidance). Two documents likely of particular interest:

* [Mindful collaboration patterns](https://github.com/symposium-dev/symposium/blob/main/symposium/mcp-server/src/guidance/main.md)
* [Coding guidelines](https://github.com/symposium-dev/symposium/blob/main/symposium/mcp-server/src/guidance/coding-guidelines.md)

We expect this guidance to evolve significantly over time!