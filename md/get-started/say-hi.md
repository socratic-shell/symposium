# Say "Hi"

The saved prompt "hi" (or "yiasou", to be appropriately Greek themed) is meant to kick off a new session. It seeds the agent with a [collaboartive prompt](../ref/collaborative-prompts.md), specifies the Sympsoium coding standards and walkthrough guidelines, and gives other base information.

If you are running in a taskspace, it will also fetch information from the Symposium app about the taskspace. However, in that case, you don't typically need to use the prompt since the Symposium app does it for you.

## Running a saved prompt

The specifics of how you run a saved prompt depend on the agent you are using.

```bash
# Example: Claude Code
/symposium:hi

# Example: Q CLI
@hi
```
