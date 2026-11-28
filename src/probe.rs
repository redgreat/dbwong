use super::{App, Error, Result, identity};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Deserialize)]
pub struct Probe {
    connection: i64,
}

pub async fn probe(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Probe>,
) -> Result<Json<Value>> {
    app.same_origin(&headers)?;
    let db = app.db().await?;
    let actor = identity(&db, &headers, true).await?;
    let id = input.connection;
    let row = db
        .query_opt(
            "SELECT engine,enabled,pgp_sym_decrypt(encrypted_dsn,$2) FROM connections WHERE id=$1 AND deleted_at IS NULL",
            &[&id, app.key.as_ref()],
        )
        .await?
        .ok_or(Error(StatusCode::NOT_FOUND))?;
    if !row.get::<_, bool>(1) {
        return Err(Error(StatusCode::CONFLICT));
    }
    if row.get::<_, String>(0) != "postgres" {
        return Err(Error(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let begin = db.query_one(
        "INSERT INTO admin_events(actor,action,subject,connection_id) VALUES($1,'probe.start',$2,$2) RETURNING id",
        &[&actor,&id]
    ).await?;
    let request: i64 = begin.get(0);
    let outcome = match tokio::time::timeout(Duration::from_secs(5), check(row.get(2))).await {
        Ok(true) => "probe.ok",
        Ok(false) => "probe.failed",
        Err(_) => "probe.timeout",
    };
    db.execute(
        "INSERT INTO admin_events(actor,action,subject,connection_id) VALUES($1,$2,$3,$4)",
        &[&actor, &outcome, &request, &id],
    )
    .await?;
    Ok(Json(json!({"outcome":outcome,"request":request})))
}

async fn check(dsn: &str) -> bool {
    let Ok(config) = dsn.parse::<tokio_postgres::Config>() else {
        return false;
    };
    let Ok(tls) = native_tls::TlsConnector::new() else {
        return false;
    };
    let Ok((client, connection)) = config
        .connect(postgres_native_tls::MakeTlsConnector::new(tls))
        .await
    else {
        return false;
    };
    // Drive the socket only for this request. Dropping the outer timeout drops both futures.
    tokio::select! {
        result = client.simple_query("SELECT 1") => result.is_ok(),
        _ = connection => false,
    }
}
