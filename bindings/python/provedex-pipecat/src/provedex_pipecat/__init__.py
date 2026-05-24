"""Provedex binding for Pipecat voice agent pipelines."""

from .config import ProvedexConfig
from .processor import ProvedexFrameProcessor

__version__ = "0.1.0"
__all__ = ["ProvedexConfig", "ProvedexFrameProcessor"]
