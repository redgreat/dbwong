use super::{App, Error, Result, identity};
use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

const PAGE_SIZE: usize = 25;

#[derive(Deserialize)]
pub struct Filter {
    before: Option<i64>,
}

#[derive(Serialize)]
struct Event {
    id: i64,
    occurred_at: String,
    actor: String,
    action: String,
    subject: i64,
    connection: Option<String>,
}

#[derive(Serialize)]
pub struct AuditPage {
    events: Vec<Event>,
    next_before: Option<i64>,
}

pub async fn list(
    State(app): State<App>,
    headers: HeaderMap,
    Query(filter): Query<Filter>,
) -> Result<Json<AuditPage>> {
    let db = app.db().await?;
    identity(&db, &headers, true).await?;
    if filter.before.is_some_and(|id| id <= 0) {
        return Err(Error(StatusCode::BAD_REQUEST));
    }
    let limit = (PAGE_SIZE + 1) as i64;
    let rows = db
        .query(
            "SELECT e.id, trim(both chr(34) from to_json(e.occurred_at)::text),
                a.name, e.action, e.subject, c.name
         FROM admin_events e
         JOIN accounts a ON a.id=e.actor
         LEFT JOIN connections c ON c.id=e.connection_id
         WHERE e.id < $1 ORDER BY e.id DESC LIMIT $2",
            &[&filter.before.unwrap_or(i64::MAX), &limit],
        )
        .await?;
    let has_more = rows.len() > PAGE_SIZE;
    let events: Vec<_> = rows
        .into_iter()
        .take(PAGE_SIZE)
        .map(|row| Event {
            id: row.get(0),
            occurred_at: row.get(1),
            actor: row.get(2),
            action: row.get(3),
            subject: row.get(4),
            connection: row.get(5),
        })
        .collect();
    let next_before = has_more.then(|| events[PAGE_SIZE - 1].id);
    Ok(Json(AuditPage {
        events,
        next_before,
    }))
}
