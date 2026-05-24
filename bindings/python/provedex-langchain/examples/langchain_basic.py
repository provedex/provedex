"""Minimal LangChain LCEL pipeline with Provedex signing.

Run a local provedex-agent before starting:
    provedex-agent --rate-limit-off &
"""

import asyncio
import os

from langchain_core.language_models.fake_chat_models import FakeListChatModel
from langchain_core.prompts import ChatPromptTemplate

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig


async def main() -> None:
    cfg = ProvedexConfig(
        agent_url=os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765"),
        agent_id="example-langchain-agent",
        model_id="fake-list",
        session_id="example-session-001",
    )
    handler = ProvedexCallbackHandler(config=cfg)

    # Replace FakeListChatModel with ChatOpenAI(model="gpt-4o") or any real LLM.
    llm = FakeListChatModel(responses=["Hello back."])
    prompt = ChatPromptTemplate.from_template("Say hi to {name}.")
    chain = prompt | llm

    async with handler.session("example-request"):
        await chain.ainvoke({"name": "world"}, config={"callbacks": [handler]})

    await handler.stop()
    print(f"signed={handler.signed_total} dropped={handler.dropped_total}")


if __name__ == "__main__":
    asyncio.run(main())
