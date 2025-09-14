# Symposium Demo Script (Alternative Version)

*Target: 5-minute screencast*

## Opening: Rust + Socratic Shell Mission (1 minute)

*Camera just shows me, hanging out on my couch.*

Hi! My name is Niko Matsakis. If you don't know me, I'm one of the lead designers of the Rust programming language and I'm also a Senior Principal Engineer. This video is meant as an introduction to the Socratic Shell github org -- and to show off the first project we've been working on!

Well, I say "we", but at the moment, the Socratic Shell project is just me -- but that's why I'm making this video! I am interesting in creating an open-source org dedicated to exploring the potential for AI to make programming easier, more collaborative, and -- well -- just plain *joyful*.

I know there's a lot of skepticism about AI. I've got concerns too. I'm worried about power usage, for example, though I have faith in the smart people banging on the problem that things will get more efficient. And I don't love hallucinations, though I don't honestly find that to be that different from what you see on your average web search these days. If it ain't Wikipedia, I don't trust it.

But for me, using AI tools has just been *tremendously* empowering. It's not just that I get more done, it's that I have a lot more fun doing it.

*[Open terminal/Q CLI]*

Now, I want to show you what I mean by collaborative programming, but first I need to figure out what would make a good demo. Let me think through this with my AI partner...

## Live Brainstorming Session (2 minutes)

*[Start conversation with AI in terminal]*

[Your actual brainstorming conversation about demo tasks]

Me: "So I'm trying to figure out what would make a compelling demo for Symposium. I want to show the collaborative aspect, but I need a concrete task that's not too complex but shows real value..."

*[Show real back-and-forth with the AI]*

[AI suggests various options - badge indicators, search enhancements, UI improvements]

[You explore the pros and cons of each]

Me: "The badge indicator idea is interesting... You see, when an agent hits some kind of problem, it can use this 'signal user' tool to let you know that it wants help. I'd like to extend the app to show a badge on the dock icon so we know that we have been signalled."

[AI responds with implementation thoughts, scope considerations]

*[Reach the decision point]*

Me: "Yeah, that badge indicator idea sounds perfect - visual, useful, good scope for a demo. And maybe I could pair it with brainstorming about enhancing the IDE search context..."

## The Meta Moment (30 seconds)

*[Pause the conversation]*

Alright, that sounds like a good task. Let's spawn a taskspace for it.

*[Launch Symposium, show empty panel]*

And THIS is exactly what Symposium does! Symposium is one of two projects in the Socratic Shell org. It's an OS X app, though I'd love to have some folks help me port it to windows or linux (hint, hint).

You can think of it like a "meta IDE". When you run symposium, you point it at a git repository. It will create a checkout and then help you coordinate various worktrees.

*[Open the Symposium.symposium project]*

In this case I'm going to be using Symposium on itself - we just planned this task together, and now we're going to execute it using the tool itself.

These subdirectories are called *taskspaces*. Each taskspace is a git worktree exploring a particular task-- but each taskspace also has an AI agent in it, pursuing some task.

## Execute the Plan (1.5 minutes)

*[Click new taskspace button]*

So, with symposium, you can really easily start up a new taskspace. I'll create one for the badge indicator task we just brainstormed.

*[Create taskspace with the badge implementation task]*

So, for this taskspace, I'm going to be adding that feature we just discussed - showing a badge on the dock icon when agents signal for help.

*[Set up the taskspace with the specific prompt]*

*[Click spawn taskspace again for the brainstorming task]*

And I'll make a second taskspace for that other idea - exploring how to enhance the IDE search with automatic context.

*[Show both taskspaces in panel]*

Now you can see I have these two taskspaces going. When I click on them, a VSCode window opens up in that new taskspace with an AI agent ready to work on the task.

*[Show the agents starting to work, maybe some progress updates]*

[Continue with implementation, walkthrough, interaction as the agents work on the tasks we just collaboratively planned]

---

## Notes for Yourself

- Emphasize the recursive nature: planning the demo → using the tool → executing the plan
- Show genuine collaborative thinking in the brainstorming section
- Make the transition from brainstorming to Symposium feel natural and revelatory
- Keep the energy of "we just figured this out together and now we're doing it"
