# Symposium Demo Script (Alternative Version)

*Target: 5-minute screencast*

## Opening: Rust + Symposium Mission (1 minute)

Hi! My name is Niko Matsakis. If you don't know me, I'm one of the lead designers of the Rust programming language. I'm also a Senior Principal Engineer at Amazon. But I'm not here to talk about either of those. I'm here to talk about *Symposium*, a new open-source project I'm kicking off.

Symposium started because, as I was working with AI agents, I found there was a kind of gap. I wanted better support for the iterative, collaborative way I was working, where me and the agent are working together to solve tricky problems. And I wanted them to do a better job with generating code, especially Rust code. And I wanted these tools to be unopinionated. I wanted to be able to try out different agents, work with different editors, etc.

I'll demo what I've got so far in a second, and I think it's pretty cool, but it's also not what I'm most excited about. What I'm most excited about is idea of a **community of developers keen to explore the best ways to use AI**. An open-source community where those developers will collaborate to build tools supporting those workflows. If we do our job right, I'd like those tools to go further, creating conventions that allow library authors the ability to influence and steer AI agents -- such as telling them how to upgrade your library, what patterns work best, or what patterns to avoid.

I know that there are a lot of AI skeptics out there, and there are good reasons for that. It uses a lot of power. It hallucinates. It gets stuck on complex problems. And a lot of companies, including my employer, are adding chatbots into every nook and cranny of their products. All of these things are truth.

And yet, it's *also* true that AI has unlocked something for me. Suddenly, I feel free to take on any project I want to pursue. Now, if I want to dive into a new area, I can fire up Claude -- I've mostly worked with Claude, but I imagine the same is true for other models too -- and I know we'll be able to get started. I can help Claude when they hallucinate and get stuck. And when we hit problems that neither of us know how to solve, we can dig into it together.

In short, to me, AI is all about empowerment. And that has me excited, since empowering developers is what I love the most.

## Demo

*View switches over to VSCode*

OK, enough talking, let's me show you how Symposium works. Remember I was telling you that I have adopted an iterative, collaborative workflow? Well, that means that the first thing I generally do is brainstorming. I approach this more or less like I would talking to a human.

*Niko types:* Hi Claude, how are you?

You see how I'm starting with a bit of pleasantries? Maybe that seems weird to you, but it serves a purpose. Remember, LLMs are, fundamentally, completion engines. This means they take their cues from the tone that you use with them. So if you treat them kindly, they'll return the favor.

*Claude responds:* ... 

*Niko types (and reads aloud as he does so):* We're actually live here! I'm preparing a demo of Symposium. I was hoping you could help me, I want to show off some of its features. I'm wondering what might make the point most effectively. What do you think?

*Claude responds:* ... 

*Niko types (and reads aloud as he does so):* Oh, I have an idea. I think one of the coolest things about Symposium is the way that it helps you to learn new things, so let's focus on that. How about we just show a bit of how the codebase works?

*Claude responds:* ... 

*Niko types:* Yeah, how about this. I'm a sucker for meta-recurisve nonsense. So how about you present me a walkthrough showing what happens when you present a walkthrough. =) Start from the point where the MCP tool is invoked and show how the data passes through the code and into the VSCode extension to be rendered. 

*Claude shows a walkthrough:* ... 

So, "walkthroughs" are one of the core parts of symposium. They are an MCP tool where the LLM prepares markdown for you. This markdown gets rendered in the VSCode extension. And it can include some extra things, such as mermaid, or comments on particular lines of code. When I'm diving into a new code base, walkthroughs are a key part of how I do it now -- I ask the LLM to talk me through what I want to learn.

And here's the cool part -- walkthroughs are also interactive. For example, I can click on Reply here, and you see that it inserts this little XML Tag into my session. When I press enter, the agent is going to look this up, figure out what I am replying to, and compose a sensible reply.

You can also use this to "tag" parts of your code by selecting it and clicking Ask Socratic Shell. Same idea.

*Claude makes edits:* ...

