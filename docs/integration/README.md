# Integration guides

How to plug Provedex into existing AI agent stacks.

Planned:

- `python.md` - using the `provedex` PyPI package from a FastAPI / asyncio app.
- `node.md` - using `@provedex/core` from a TypeScript service.
- `langchain.md` - LangChain callback handler that emits a signed event per chain step.
- `letta.md` - Letta tool-call hook that wraps every tool with a signed event.
- `voice-agents.md` - generic STT -> LLM -> TTS pipeline integration.
- `aggregator-forwarding.md` - pushing signed events from the SDK to the hosted aggregator.

Each guide ends with a "verify" section showing the customer how to prove their integration emits real signed events.
