//! The seven AgentEvent variants as typed Python factories, plus a from_dict
//! reconstruction path. The variant set is locked to provedex-core; there is no
//! binding-only event.

use pyo3::prelude::*;
use pyo3::types::PyModule;
use pythonize::depythonize;

use provedex_core::AgentEvent as CoreEvent;

use crate::errors::signed_err;

/// Opaque handle around a provedex-core AgentEvent. Built only via the factory
/// functions or from_dict; Python never constructs the tagged JSON by hand.
// skip_from_py_object: this handle is only ever passed by reference (&AgentEvent),
// never extracted by value, so the Clone-derived FromPyObject impl is unused. In
// pyo3 0.29 that impl became opt-in; opting out keeps behavior and silences the
// deprecation.
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct AgentEvent {
    pub(crate) inner: CoreEvent,
}

#[pymethods]
impl AgentEvent {
    fn __repr__(&self) -> String {
        // Tag only; payloads can carry transcripts we do not want in a repr.
        let tag = match &self.inner {
            CoreEvent::SessionStarted { .. } => "SessionStarted",
            CoreEvent::UtteranceCaptured { .. } => "UtteranceCaptured",
            CoreEvent::ToolCalled { .. } => "ToolCalled",
            CoreEvent::ToolReturned { .. } => "ToolReturned",
            CoreEvent::ModelInvoked { .. } => "ModelInvoked",
            CoreEvent::UtteranceSpoken { .. } => "UtteranceSpoken",
            CoreEvent::SessionEnded { .. } => "SessionEnded",
        };
        format!("AgentEvent(type='{tag}')")
    }
}

#[pyfunction]
fn session_started(agent_id: String, model_id: String, session_id: String) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::SessionStarted {
            agent_id,
            model_id,
            session_id,
        },
    }
}

#[pyfunction]
fn utterance_captured(
    audio_sha256: String,
    transcript: String,
    lang: String,
    duration_ms: u64,
) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::UtteranceCaptured {
            audio_sha256,
            transcript,
            lang,
            duration_ms,
        },
    }
}

#[pyfunction]
fn tool_called(
    tool_name: String,
    args_sha256: String,
    args_redacted: Bound<'_, PyAny>,
) -> PyResult<AgentEvent> {
    let args_redacted = depythonize(&args_redacted)
        .map_err(|e| crate::errors::SigningError::new_err(e.to_string()))?;
    Ok(AgentEvent {
        inner: CoreEvent::ToolCalled {
            tool_name,
            args_sha256,
            args_redacted,
        },
    })
}

#[pyfunction]
fn tool_returned(
    tool_name: String,
    result_sha256: String,
    latency_ms: u64,
    success: bool,
) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::ToolReturned {
            tool_name,
            result_sha256,
            latency_ms,
            success,
        },
    }
}

#[pyfunction]
fn model_invoked(
    model_id: String,
    prompt_sha256: String,
    response_sha256: String,
    prompt_tokens: u32,
    response_tokens: u32,
) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::ModelInvoked {
            model_id,
            prompt_sha256,
            response_sha256,
            prompt_tokens,
            response_tokens,
        },
    }
}

#[pyfunction]
fn utterance_spoken(text_sha256: String, text: String, audio_sha256: String) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::UtteranceSpoken {
            text_sha256,
            text,
            audio_sha256,
        },
    }
}

#[pyfunction]
fn session_ended(reason: String, summary_sha256: String) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::SessionEnded {
            reason,
            summary_sha256,
        },
    }
}

/// Rebuild an AgentEvent from its tagged `{"type", "payload"}` mapping. Rejects
/// any shape that is not one of the seven core variants.
#[pyfunction]
fn from_dict(value: Bound<'_, PyAny>) -> PyResult<AgentEvent> {
    let json: serde_json::Value =
        depythonize(&value).map_err(|e| crate::errors::SigningError::new_err(e.to_string()))?;
    let inner: CoreEvent = serde_json::from_value(json)
        .map_err(|e| signed_err(provedex_core::SignedError::Json(e)))?;
    Ok(AgentEvent { inner })
}

pub(crate) fn build(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let m = PyModule::new(py, "events")?;
    m.add_class::<AgentEvent>()?;
    m.add_function(wrap_pyfunction!(session_started, &m)?)?;
    m.add_function(wrap_pyfunction!(utterance_captured, &m)?)?;
    m.add_function(wrap_pyfunction!(tool_called, &m)?)?;
    m.add_function(wrap_pyfunction!(tool_returned, &m)?)?;
    m.add_function(wrap_pyfunction!(model_invoked, &m)?)?;
    m.add_function(wrap_pyfunction!(utterance_spoken, &m)?)?;
    m.add_function(wrap_pyfunction!(session_ended, &m)?)?;
    m.add_function(wrap_pyfunction!(from_dict, &m)?)?;
    parent.add_submodule(&m)?;
    // Register in sys.modules so `import provedex.events` resolves, not just
    // attribute access through the parent package.
    py.import("sys")?
        .getattr("modules")?
        .set_item("provedex.events", &m)?;
    Ok(())
}
