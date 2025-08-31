# Code Walkthroughs and Ask Socratic Shell

Dialectic makes working on code with AI into a very organic process. Think of the AI as a "pair programmer", working with you to help you figure out what you want to build -- not an oracle who knows everything nor a servant, but a tool to help *you* understand what's going on more easily.

Dialectic adds two core tools that make this workflow much easier:

* Presenting a code walkthrough;
* The "Ask Socratic Shell" feature for highlighting particular pieces of the code.

## Code walkthroughs

With dialectic, you can ask your AI assistant to explain code using a code walkthrough. This can be useful when it has just finished authoring up some code:

```
You: "Present a review of the authentication system you just built"
```

...but it's also useful when you are trying to understand how something works:

```
You: "Walk me through what happens from the point when new message arrives until the database is modified. I am particularly interested in how we decide the value of the USERNAME column."
```

When you issue commands like these, the assistant will prepare a walkthrough for you and display it as a structured document. It will pop up a second pane amongst your editors. It will include links that so you can browse the code it is talking about and compare it to the review.

## Ask Socratic Shell

As you are walking through the code, you will naturally have questions -- or maybe you see a section of the code that looks wrong. The "Ask Socratic Shell" feature can help you with this. Just select the code in question and you will see a lightbulb appear. Near the top should be "Ask Socratic Shell". When you select this, the Socratic Shell will identify which terminal your CLI assistant is running in and insert some text to help it identify what code you are talking about. You can then continue and type your question or suggestion:


```
You: *selects ask socratic shell*

Socratic Shell: *inserts `<context>looking at this code from md/process-request.rs:7:1-59</context>` into your terminal window

You: "This function isn't accounting for what happens when the user cancels the request. Modify it to present an error dialog."
```

## Frequently asked questions

See the [FAQ](./faq.md) for troubleshooting advice and more details about using these features.

