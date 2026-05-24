import subprocess
from pathlib import Path
from typing import TypedDict

import pytest
from langchain_core.language_models.fake_chat_models import FakeListChatModel
from langchain_core.prompts import ChatPromptTemplate

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig

REPO_ROOT = Path(__file__).resolve().parents[4]


def _provedex_cli() -> Path:
    cli = REPO_ROOT / "target" / "release" / "provedex"
    if not cli.exists():
        subprocess.run(
            ["cargo", "build", "--release", "-p", "provedex-cli"],
            cwd=REPO_ROOT,
            check=True,
        )
    return cli


@pytest.mark.integration
async def test_langchain_pipeline_produces_valid_ledger(agent):
    cfg = ProvedexConfig(
        agent_url=agent["base_url"],
        session_id="int-langchain",
        agent_id="int-langchain-agent",
        model_id="fake-list",
    )
    handler = ProvedexCallbackHandler(config=cfg)

    llm = FakeListChatModel(responses=["hi there"])
    prompt = ChatPromptTemplate.from_template("Say hi: {topic}")
    chain = prompt | llm

    async with handler.session("test-request"):
        await chain.ainvoke({"topic": "ducks"}, config={"callbacks": [handler]})

    await handler.stop()

    result = subprocess.run(
        [str(_provedex_cli()), "verify", "--ledger", str(agent["ledger"])],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, (
        f"provedex verify failed: stdout={result.stdout} stderr={result.stderr}"
    )
    # SessionStarted, ModelInvoked, SessionEnded
    assert handler.signed_total >= 3
    assert handler.dropped_total == 0


@pytest.mark.integration
async def test_langgraph_pipeline_produces_valid_ledger(agent):
    from langgraph.graph import END, START, StateGraph

    class State(TypedDict):
        answer: str

    llm = FakeListChatModel(responses=["graph response"])

    async def node_a(state: State, config) -> State:
        resp = await llm.ainvoke("hi", config=config)
        return {"answer": resp.content}

    graph_builder = StateGraph(State)
    graph_builder.add_node("a", node_a)
    graph_builder.add_edge(START, "a")
    graph_builder.add_edge("a", END)
    graph = graph_builder.compile()

    cfg = ProvedexConfig(
        agent_url=agent["base_url"],
        session_id="int-langgraph",
        agent_id="int-langgraph-agent",
        model_id="fake-list",
    )
    handler = ProvedexCallbackHandler(config=cfg)

    async with handler.session("graph-run"):
        await graph.ainvoke({"answer": ""}, config={"callbacks": [handler]})

    await handler.stop()

    result = subprocess.run(
        [str(_provedex_cli()), "verify", "--ledger", str(agent["ledger"])],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, (
        f"provedex verify failed: stdout={result.stdout} stderr={result.stderr}"
    )
    assert handler.signed_total >= 3
    assert handler.dropped_total == 0
