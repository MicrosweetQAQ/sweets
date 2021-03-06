use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use tokio_postgres::Client;

use crate::error::Error;
use crate::event::{Event, EventId, EventRepositoryExt};
use crate::result::Result;
use crate::sql::where_builder::WhereBuilder;

pub struct PostgresEventRepository {
    client: Arc<Client>,
}

impl PostgresEventRepository {
    pub fn new(client: Arc<Client>) -> Self {
        PostgresEventRepository { client }
    }
}

#[async_trait]
impl EventRepositoryExt for PostgresEventRepository {
    async fn search(
        &self,
        topic: Option<&String>,
        code: Option<&String>,
        from: Option<&DateTime<Utc>>,
        to: Option<&DateTime<Utc>>,
    ) -> Result<Vec<Event>> {
        let (sql, params) = WhereBuilder::new()
            .add_param_opt("topic = $$", &topic, topic.is_some())
            .add_param_opt("code = $$", &code, code.is_some())
            .add_param_opt("timestamp >= $$", &from, from.is_some())
            .add_param_opt("timestamp <= $$", &to, to.is_some())
            .build();

        let rows = self
            .client
            .query(
                &format!(
                    "SELECT * FROM events
                    {}
                    ORDER BY timestamp ASC",
                    sql,
                ) as &str,
                &params,
            )
            .await
            .map_err(|err| Error::not_found("event").wrap_raw(err))?;

        let mut events = Vec::new();

        for row in rows.into_iter() {
            let id: Vec<u8> = row.get("id");
            let topic: String = row.get("topic");
            let code: String = row.get("code");
            let timestamp: DateTime<Utc> = row.get("timestamp");
            let payload: Value = row.get("payload");

            events.push(Event::build(
                EventId::from(id),
                topic,
                code,
                timestamp,
                payload,
            ));
        }

        Ok(events)
    }

    async fn save(&self, event: &Event) -> Result<()> {
        self.client
            .execute(
                "INSERT INTO events (
                    id,
                    topic,
                    code,
                    timestamp,
                    payload
                ) VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5
                )",
                &[
                    &event.id().as_ref(),
                    &event.topic(),
                    &event.code(),
                    &event.timestamp(),
                    &event.payload(),
                ],
            )
            .await
            .map_err(|err| Error::new("event", "create").wrap_raw(err))?;

        Ok(())
    }
}
