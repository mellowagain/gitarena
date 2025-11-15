//! # Server Sent Events
//! Please note the [limits] that apply when not sent over HTTP/2.
//! Adapted from https://github.com/arve0/actix-sse/ and https://github.com/actix/examples/blob/master/server-sent-events
//!
//! [limits]: https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#sect1

use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use actix_web::web::{Bytes, Data};
use anyhow::Result;
use derive_more::{Deref, Display};
use futures::Stream;
use futures_locks::RwLock;
use log::debug;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tracing::instrument;

pub(crate) const SSE_BUFFER_SIZE: usize = 512;

#[derive(Default)]
pub(crate) struct Broadcaster {
    clients: Vec<(Sender<Bytes>, Category)>,
}

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Hash)]
pub(crate) enum Category {
    #[display(fmt = "log")]
    AdminLog,
}

impl Broadcaster {
    pub(crate) fn new() -> Data<RwLock<Self>> {
        let data = Data::new(RwLock::new(Broadcaster::default()));

        spawn_ping_task(data.clone());

        data
    }

    #[instrument]
    pub(crate) async fn new_client(&mut self, category: Category) -> Result<SseClient> {
        let (tx, rx) = channel(SSE_BUFFER_SIZE);
        tx.send(Bytes::from("data: connected\n\n")).await?;

        debug!("New client subscribed to category {}", category);

        self.clients.push((tx, category));
        Ok(SseClient(rx))
    }

    /// Sends a message to all clients subscribed to a specific [Category]
    #[instrument]
    pub(crate) fn send(&self, category: Category, message: &str) {
        let sse_message = format!("event: {}\ndata: {}\n\n", category, message);
        let bytes = Bytes::from(sse_message);

        debug!("Broadcasting in category {}: \"{}\"", category, message);

        for (client, _) in self.clients.iter().filter(|(_, c)| *c == category) {
            // Errors would only occur if the client disconnected
            // As disconnected clients would be removed by [remove_state_clients] every 10 seconds, ignoring the error here is OK
            let _ = client.try_send(bytes.clone());
        }
    }

    /// Removes clients which we are unable to send a ping to
    /// This method should be called by a tokio task around every 10 seconds
    #[instrument]
    async fn remove_stale_clients(&mut self) {
        self.clients.retain(|(client, category)| {
            // This will fail if the buffer is full or the client is disconnected
            // If the buffer is full the client has not recv'd for a while which means it probably disconnected
            client
                .try_send(Bytes::from("event: ping\ndata: pong!\n\n"))
                .map_or_else(
                    |err| {
                        debug!("Disconnecting a client subscribed to {}: {}", category, err);
                        false
                    },
                    |_| true,
                )
        });
    }

    pub(crate) fn len(&self) -> usize {
        self.clients.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }
}

impl Debug for Broadcaster {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut map = HashMap::new();

        for (_, category) in &self.clients {
            let count = map.entry(*category).or_insert(0_usize);
            *count += 1;
        }

        Debug::fmt(&map, f)
    }
}

#[derive(Debug, Deref)]
pub(crate) struct SseClient(Receiver<Bytes>);

impl Stream for SseClient {
    type Item = actix_web::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_recv(context) {
            Poll::Ready(Some(value)) => Poll::Ready(Some(Ok(value))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Spawns a task which will ping all clients every 10 seconds in order to keep the connection alive
fn spawn_ping_task(data: Data<RwLock<Broadcaster>>) {
    let mut interval = tokio::time::interval(Duration::new(10, 0));

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            data.write().await.remove_stale_clients().await;
        }
    });
}
