use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use futures::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::state::AppState;

pub async fn events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.broadcast.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(evt) => Some(Ok::<_, Infallible>(
            Event::default()
                .event("signed")
                .json_data(evt)
                .unwrap_or_else(|_| Event::default().data("serialization_error")),
        )),
        Err(_) => None,
    });
    let backlog = state.ledger().read_all().unwrap_or_default();
    let backlog_stream = futures::stream::iter(backlog.into_iter().map(|evt| {
        Ok::<_, Infallible>(
            Event::default()
                .event("signed")
                .json_data(evt)
                .unwrap_or_else(|_| Event::default().data("serialization_error")),
        )
    }));
    let combined: Box<dyn Stream<Item = Result<Event, Infallible>> + Send + Unpin> =
        Box::new(backlog_stream.chain(stream));
    Sse::new(combined).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
