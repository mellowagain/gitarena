use crate::sse::Category;
use crate::Broadcaster;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Debug;

use actix_web::web::Data;
use chrono::Utc;
use derive_more::{Deref, DerefMut};
use futures_locks::RwLock;
use log::warn;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

#[derive(Debug)]
pub(crate) struct AdminPanelLayer {
    broadcaster: Data<RwLock<Broadcaster>>,
}

impl AdminPanelLayer {
    pub(crate) fn new(broadcaster: Data<RwLock<Broadcaster>>) -> Self {
        AdminPanelLayer { broadcaster }
    }
}

impl<S: Subscriber> Layer<S> for AdminPanelLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let level = *event.metadata().level();

        let sse = event
            .metadata()
            .module_path()
            .map_or_else(|| false, |module| module == "gitarena::sse");
        let valid = !sse || level != Level::DEBUG;

        if valid {
            match self.broadcaster.try_read() {
                Ok(broadcaster) => {
                    // Don't fill the channel if no consumer is active to prevent the channel from reaching its buffer size
                    if !broadcaster.is_empty() {
                        let mut fields = BTreeMap::new();

                        let mut visitor = AdminPanelVisitor(&mut fields);
                        event.record(&mut visitor);

                        if let Some(message) = fields.get("message") {
                            // Short the rfc 3339 timestamp to be consistent with the default log format
                            let timestamp = Utc::now().to_rfc3339();
                            let shortened = &timestamp[..26];
                            let formatted_message =
                                format!("{}Z [{}] {}", shortened, level.as_str(), message.as_str());

                            broadcaster.send(Category::AdminLog, formatted_message.as_str());
                        }
                    }
                }
                Err(err) => warn!("Failed to acquire read lock for Broadcaster: {}", err),
            }
        }
    }
}

#[derive(Debug, Deref, DerefMut)]
pub(crate) struct AdminPanelVisitor<'a>(&'a mut BTreeMap<String, String>);

impl<'a> Visit for AdminPanelVisitor<'a> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field.name().to_string(), value.to_string());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(field.name().to_string(), value.to_owned());
    }

    fn record_error(&mut self, field: &Field, _value: &(dyn Error + 'static)) {
        self.insert(field.name().to_string(), "error".to_owned());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.insert(field.name().to_string(), format!("{:?}", value));
    }
}
