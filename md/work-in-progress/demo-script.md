# Symposium Demo Script

*Target: 5-minute screencast*

## Opening Hook (30 seconds)

*Camera just shows me, hanging out on my couch.*

Hi! My name is Niko Matsakis. If you don't know me, I'm one of the lead designers of the Rust programming language and I'm also a Senior Principal Engineer. This video is meant as an introduction to the Socratic Shell github org -- and to show off the first project we've been working on!

Well, I say "we", but at the moment, the Socratic Shell project is just me -- but that's why I'm making this video! I am interesting in creating an open-source org dedicated to exploring the potential for AI to make programming easier, more collaborative, and -- well -- just plain *joyful*.

I know there's a lot of skepticism about AI. I've got concerns too. I'm worried about power usage, for example, though I have faith in the smart people banging on the problem that things will get more efficient. And I don't love hallucinations, though I don't honestly find that to be that different from what you see on your average web search these days. If it ain't Wikipedia, I don't trust it.

But for me, using AI tools has just been *tremendously* empowering. It's not just that I get more done, it's that I have a lot more fun doing it. Let me just show you!

## Introducing Symposium

*Launch Symposium, show empty panel*

I'm going to start by launching Symposium. Symposium is one of two projects in the Socratic Shell org. It's an OS X app, though I'd love to have some folks help me port it to windows or linux (hint, hint).

Anyway, you can think of it like a "meta IDE". When you run symposium, you point it at a git repository. It will create a checkout and then help you coordinate various worktrees. 

*Click the open project, explores in the Finder window*

In this case I'm going to be using Symposium on itself, so let me just open up a pre-existing project -- Symposium.symposium. You can see here that a symposium project is just a directory with a bunch of subdirectories.

*Open symposium*

These subdirectories are called *taskspaces*. Each taskspace is a git worktree exploring a particular task-- but each taskspace also has an AI agent in it, pursuing some task. Right now i've just started up, so what you are seeing here are older taskspaces I had from before. There are AI agents in these, running in the background —- though probably most of them are just blocked right now, waiting on me to do something.

So, you might be wondering, I thought this was about AI. Why am I looking at some kind of programmer's Window Manager. I mean we already have Xmonad, am I right? Well, the thing is, what I've seen is that, as you get really into AI, you start to want to do more tasks at once. The idea is that you are talking to the agent and you set it up to do some task, but then you just kind of have to wait. And while you wait, you'd like to be doing something else.

So, with symposium, you can really easily start up a new taskspace. One way is to click this button and describe the thing you want to do. LEt me go ahead and spawn a taskspace now.

*[Click new taskspace button]*

So, for this taskspace, I'm going to be adding a feature. You see, when the agent hits some kind of problem, it can use this "signal user" tool to let you know that it wants help. I'd like to extend the app to show a badge on the dock icon so we know that we have been signalled. 

*types that into the taskspace box and hits create*

OK, so, that's launching up. Now, I'm going to make a second taskspace, because I've got this other idea I want to pursue. You see, one of the things that is part of socratic shell is an MCP server that gives access to more structured operations, like IDE operations. I've been thinking about how, when I do a search, I often see the AI follow that up by fetching text *around* the search hit to get context, and I started thinkning, what if search provided a certain amount of context by default? I wanted to play around with this idea a bit.

*types that into the taskspace box and hits create*

OK, so, now you can see I have these two taskspaces going. 

* second, I hvae this cool idea I want to brainstorm.

You can see that each taskspace I've spawned shows up here with some kind of placeholder test. When I click on them, a VSCode window opens up in that new taskspace. This window is running a VSCode extension that 

[Explain what you're creating - bug fix taskspace]

*[Set up first taskspace with initial prompt]*

[Your narration about the bug fix agent]

*[Click spawn taskspace again]*

[Set up brainstorming taskspace]

*[Show both taskspaces in panel]*

[Transition to philosophy section]

## Philosophy & Parallel Work (1 minute)

*[Gesture to both active taskspaces]*

[Your thoughts on human-AI collaboration - not command-and-execute but partnership]

*[Show agents working in parallel]*

[Explain complementary strengths - vision vs analysis/implementation]

*[Maybe show progress updates in panel]*

[Transition to first milestone]

## Walkthrough & Interaction (2 minutes)

*[Bug fix agent completes first chunk]*

[React to the completion notification]

*[Agent presents walkthrough with mermaid diagram]*

[Your reaction to the walkthrough - appreciate the visual explanation]

*[Click Reply on a specific comment]*

[Express curiosity - "Hmm, this is interesting..."]

*[Type your question/concern in reply]*

[Have the actual conversation with the agent]

*[Agent responds, maybe suggests alternatives]*

[Your follow-up - maybe suggest refactoring]

*[Agent implements the refactoring]*

[React to the quick implementation]

*[Demonstrate /reply feature working]*

[Your "sheepish admission" about implementing /reply in Q CLI]

[Explain tool agnosticism - MCP, any CLI agent, etc.]

## Learning Moment (1 minute)

*[Look at Swift code in the bug fix]*

[Your story about learning Swift - concepts vs syntax]

*[Highlight a SwiftUI annotation like @StateObject]*

[Set up the "Ask Socratic Shell" moment]

*[Use Ask Socratic Shell feature]*

[Ask your question about the Swift annotation]

*[Get explanation]*

[React to the instant expertise - how this bridges learning gaps]

[Maybe mention Socratic Shell vs Symposium distinction here?]

## Meta Moment & Wrap-up (30 seconds)

*[Switch back to bug fix agent]*

[Notice agent is starting next commit]

*[Address the agent directly]*

[Your "Oh hey, you're on live TV! Say hi!" moment]

*[Agent responds]*

[React to agent's personality]

*[Switch away from Symposium to browser]*

[Show GitHub org, open issues]

*[Browse documentation briefly]*

[Talk about the broader vision]

*[Click on specific tracking issues - IntelliJ support, Emacs support]*

[Explain extensibility vision]

[Your call to action - try it out, leave feedback]

*[End screen or fade out]*

---

## Notes for Yourself

- Remember to mention productivity gains from dogfooding
- Keep energy high throughout
- Show genuine curiosity and learning
- Emphasize authenticity - this is real workflow, not contrived demo
- End with clear ways for people to get involved

## Technical Setup Reminders

- [ ] Have good Swift bug ready to demonstrate
- [ ] Ensure walkthrough tool renders mermaid beautifully  
- [ ] Test Ask Socratic Shell workflow
- [ ] Verify /reply functionality works smoothly
- [ ] Have GitHub issues ready to show
- [ ] Check that agents can respond naturally to "say hi"
