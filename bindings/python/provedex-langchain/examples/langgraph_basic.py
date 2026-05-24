"""Minimal LangGraph pipeline with Provedex signing.

Demonstrates that LangGraph users get audit coverage automatically because
LangGraph propagates LangChain callbacks for every LLM and tool call.

Run a local provedex-agent before starting:
    provedex-agent --rate-limit-off &
"""

import asyncio
import os
from typing import TypedDict

from langchain_core.language_models.fake_chat_models import FakeListChatModel
from langgraph.graph import END, START, StateGraph

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig


class State(TypedDict):
    answer: str


async def main() -> None:
    cfg = ProvedexConfig(
        agent_url=os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765"),
        agent_id="example-langgraph-agent",
        model_id="fake-list",
    )
    handler = ProvedexCallbackHandler(config=cfg)

    llm = FakeListChatModel(responses=["Hello from the graph."])

    async def respond(state: State, config) -> State:
        resp = await llm.ainvoke("greet user", config=config)
        return {"answer": resp.content}

    graph_builder = StateGraph(State)
    graph_builder.add_node("respond", respond)
    graph_builder.add_edge(START, "respond")
    graph_builder.add_edge("respond", END)
    graph = graph_builder.compile()

    async with handler.session("graph-example"):
        await graph.ainvoke({"answer": ""}, config={"callbacks": [handler]})

    await handler.stop()
    print(f"signed={handler.signed_total} dropped={handler.dropped_total}")


if __name__ == "__main__":
    asyncio.run(main())
