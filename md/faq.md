# Frequently asked questions

## The Dialectic MCP server won't load!

Are you running from a terminal inside of your IDE? You must use a terminal from within the IDE or else the MCP server can't identify which IDE window to connect to.

## When I use "Discuss in Symposium", how does it know which terminal to send the message to?

The extension tracks which terminal windows have active MCP servers. If there is exactly one, it will use that, but if there are multiple, it should give you a choice.

### When I click on a link, it pops up a window with a lot of choices. What's going on?

The links in dialectic are often based on searches (i.e., "find this code and search for `foo`). If we can't figure out which one seems best, we'll pop up a dialog so you can choose between them. Just move the cursor to the one that seems like what the AI is talking about and press enter.

## "Discuss in Symposium" is giving me an error, saying there are no active terminals.

Hmm, that does happen, even though in theory it's not supposed to. You can get things working again by quitting your CLI and restarting it (remember to "resume" your chat to avoid losing context). That will cause it to restart the MCP servers and re-establish the connection. You could also file an issue with the contents of the "Output" tab (look for Dialectic) -- that has the various logs, and it may help us figure out the problem.
