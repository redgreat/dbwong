mod audit;
mod probe;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use deadpool_postgres::{Manager, Pool};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{env, sync::Arc};
use tokio_postgres::NoTls;

#[derive(Clone)]
struct App {
    pool: Pool,
    key: Arc<String>,
    origin: Arc<String>,
}
struct Error(StatusCode);
type Result<T> = std::result::Result<T, Error>;
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}
impl From<tokio_postgres::Error> for Error {
    fn from(e: tokio_postgres::Error) -> Self {
        use tokio_postgres::error::SqlState;
        Self(match e.code() {
            Some(&SqlState::UNIQUE_VIOLATION) => StatusCode::CONFLICT,
            Some(&SqlState::CHECK_VIOLATION | &SqlState::FOREIGN_KEY_VIOLATION) => {
                StatusCode::BAD_REQUEST
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })
    }
}
impl App {
    async fn db(&self) -> Result<deadpool_postgres::Object> {
        self.pool
            .get()
            .await
            .map_err(|_| Error(StatusCode::SERVICE_UNAVAILABLE))
    }
    fn cookie(&self, value: &str, age: u32) -> String {
        let secure = if self.origin.starts_with("https://") {
            "; Secure"
        } else {
            ""
        };
        format!("dbw_session={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={age}{secure}")
    }
    fn same_origin(&self, h: &HeaderMap) -> Result<()> {
        if h.get(header::ORIGIN).and_then(|v| v.to_str().ok()) == Some(self.origin.as_str()) {
            Ok(())
        } else {
            Err(Error(StatusCode::FORBIDDEN))
        }
    }
}
fn token(h: &HeaderMap) -> Result<&str> {
    h.get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .find_map(|v| v.trim().strip_prefix("dbw_session="))
        })
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(Error(StatusCode::UNAUTHORIZED))
}
async fn identity(db: &tokio_postgres::Client, h: &HeaderMap, admin: bool) -> Result<i64> {
    let r=db.query_opt("SELECT a.id,a.admin FROM accounts a JOIN sessions s ON s.account_id=a.id WHERE a.enabled AND s.expires>now() AND s.digest=digest($1::text,'sha256')",&[&token(h)?]).await?.ok_or(Error(StatusCode::UNAUTHORIZED))?;
    if admin && !r.get::<_, bool>(1) {
        return Err(Error(StatusCode::FORBIDDEN));
    }
    Ok(r.get(0))
}
#[derive(Deserialize)]
struct Login {
    name: String,
    password: String,
}
fn password_ok(s: &str) -> bool {
    (12..=72).contains(&s.len()) && !s.contains('\0')
}
async fn login(State(a): State<App>, h: HeaderMap, Json(v): Json<Login>) -> Result<Response> {
    a.same_origin(&h)?;
    if !password_ok(&v.password) {
        return Err(Error(StatusCode::UNAUTHORIZED));
    }
    let db = a.db().await?;
    let row=db.query_opt("WITH who AS (SELECT id FROM accounts WHERE enabled AND name=$1 AND password_hash=crypt($2,password_hash) FOR SHARE), secret AS MATERIALIZED (SELECT encode(gen_random_bytes(32),'hex') AS value), added AS (INSERT INTO sessions SELECT digest(value,'sha256'),id,now()+make_interval(hours => session_hours) FROM who,secret,app_settings RETURNING account_id) SELECT value, session_hours FROM secret,added,app_settings",&[&v.name,&v.password]).await?.ok_or(Error(StatusCode::UNAUTHORIZED))?;
    Ok((
        [(
            header::SET_COOKIE,
            a.cookie(row.get(0), row.get::<_, i32>(1) as u32 * 3600),
        )],
        StatusCode::OK,
    )
        .into_response())
}
async fn logout(State(a): State<App>, h: HeaderMap) -> Result<Response> {
    a.same_origin(&h)?;
    a.db()
        .await?
        .execute(
            "DELETE FROM sessions WHERE digest=digest($1::text,'sha256')",
            &[&token(&h)?],
        )
        .await?;
    Ok(([(header::SET_COOKIE, a.cookie("", 0))], StatusCode::OK).into_response())
}
async fn catalog(State(a): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    let db = a.db().await?;
    let id = identity(&db, &h, false).await?;
    let rows=db.query("SELECT c.id,c.name,c.engine,c.environment FROM connections c JOIN connection_read_grants g ON c.id=g.connection_id WHERE c.enabled AND g.account_id=$1 ORDER BY c.name",&[&id]).await?;
    Ok(Json(json!(rows.iter().map(connection).collect::<Vec<_>>())))
}
fn connection(r: &tokio_postgres::Row) -> Value {
    json!({"id":r.get::<_,i64>(0),"name":r.get::<_,String>(1),"engine":r.get::<_,String>(2),"environment":r.get::<_,String>(3)})
}
async fn overview(State(a): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    let db = a.db().await?;
    identity(&db, &h, true).await?;
    let session_hours: i32 = db
        .query_one(
            "SELECT session_hours FROM app_settings WHERE singleton",
            &[],
        )
        .await?
        .get(0);
    let accounts = db
        .query(
            "SELECT id,name,admin,enabled FROM accounts ORDER BY name",
            &[],
        )
        .await?;
    let connections = db
        .query(
            "SELECT id,name,engine,environment,enabled FROM connections ORDER BY name",
            &[],
        )
        .await?;
    let grants = db
        .query(
            "SELECT account_id,connection_id FROM connection_read_grants ORDER BY 1,2",
            &[],
        )
        .await?;
    Ok(Json(
        json!({"session_hours":session_hours,"accounts":accounts.iter().map(|r|json!({"id":r.get::<_,i64>(0),"name":r.get::<_,String>(1),"admin":r.get::<_,bool>(2),"enabled":r.get::<_,bool>(3)})).collect::<Vec<_>>(),"connections":connections.iter().map(|r| { let mut summary=connection(r); summary["enabled"]=json!(r.get::<_,bool>(4)); summary }).collect::<Vec<_>>(),"grants":grants.iter().map(|r|(r.get::<_,i64>(0),r.get::<_,i64>(1))).collect::<Vec<_>>()}),
    ))
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Change {
    EditConnection {
        id: i64,
        name: String,
        environment: String,
        enabled: bool,
        dsn: Option<String>,
    },
    AccountStatus {
        account: i64,
        enabled: bool,
    },
    ResetPassword {
        account: i64,
        password: String,
    },
    SessionHours {
        hours: i32,
    },
    Account {
        name: String,
        password: String,
    },
    Connection {
        name: String,
        engine: String,
        environment: String,
        dsn: String,
    },
    Grant {
        account: i64,
        connection: i64,
        allow: bool,
    },
}
async fn change(State(a): State<App>, h: HeaderMap, Json(v): Json<Change>) -> Result<StatusCode> {
    a.same_origin(&h)?;
    let mut db = a.db().await?;
    let actor = identity(&db, &h, true).await?;
    let tx = db.transaction().await?;
    let (action, subject, conn) = match v {
        Change::AccountStatus { account, enabled } => {
            let changed = tx
                .execute(
                    "UPDATE accounts SET enabled=$2 WHERE id=$1 AND NOT admin",
                    &[&account, &enabled],
                )
                .await?;
            if changed == 0 {
                return Err(Error(StatusCode::NOT_FOUND));
            }
            if !enabled {
                tx.execute("DELETE FROM sessions WHERE account_id=$1", &[&account])
                    .await?;
            }
            (
                if enabled {
                    "account.enable"
                } else {
                    "account.disable"
                },
                account,
                None,
            )
        }
        Change::ResetPassword { account, password } => {
            if !password_ok(&password) {
                return Err(Error(StatusCode::BAD_REQUEST));
            }
            let changed = tx.execute("UPDATE accounts SET password_hash=crypt($2,gen_salt('bf',12)) WHERE id=$1 AND NOT admin", &[&account,&password]).await?;
            if changed == 0 {
                return Err(Error(StatusCode::NOT_FOUND));
            }
            tx.execute("DELETE FROM sessions WHERE account_id=$1", &[&account])
                .await?;
            ("account.reset_password", account, None)
        }
        Change::SessionHours { hours } => {
            tx.execute(
                "UPDATE app_settings SET session_hours=$1 WHERE singleton",
                &[&hours],
            )
            .await?;
            ("settings.session_hours", i64::from(hours), None)
        }
        Change::Account { name, password } => {
            if !password_ok(&password) {
                return Err(Error(StatusCode::BAD_REQUEST));
            }
            let row=tx.query_one("INSERT INTO accounts(name,password_hash) VALUES(trim($1),crypt($2,gen_salt('bf',12))) RETURNING id",&[&name,&password]).await?;
            ("account.create", row.get(0), None)
        }
        Change::EditConnection {
            id,
            name,
            environment,
            enabled,
            dsn,
        } => {
            let updated = tx
                .execute(
                    "UPDATE connections SET name=trim($2), environment=$3, enabled=$4,
                 encrypted_dsn=CASE WHEN nullif($5::text,'') IS NULL THEN encrypted_dsn
                 ELSE pgp_sym_encrypt($5,$6,'cipher-algo=aes256') END WHERE id=$1",
                    &[&id, &name, &environment, &enabled, &dsn, a.key.as_ref()],
                )
                .await?;
            if updated == 0 {
                return Err(Error(StatusCode::NOT_FOUND));
            }
            ("connection.edit", id, Some(id))
        }
        Change::Connection {
            name,
            engine,
            environment,
            dsn,
        } => {
            if dsn.trim().is_empty() {
                return Err(Error(StatusCode::BAD_REQUEST));
            }
            let row=tx.query_one("INSERT INTO connections(name,engine,environment,encrypted_dsn) VALUES(trim($1),$2,$3,pgp_sym_encrypt($4,$5,'cipher-algo=aes256')) RETURNING id",&[&name,&engine,&environment,&dsn,a.key.as_ref()]).await?;
            ("connection.create", row.get(0), Some(row.get(0)))
        }
        Change::Grant {
            account,
            connection,
            allow,
        } => {
            if allow {
                tx.execute(
                    "INSERT INTO connection_read_grants VALUES($1,$2) ON CONFLICT DO NOTHING",
                    &[&account, &connection],
                )
                .await?;
            } else {
                tx.execute(
                    "DELETE FROM connection_read_grants WHERE account_id=$1 AND connection_id=$2",
                    &[&account, &connection],
                )
                .await?;
            }
            (
                if allow { "read.grant" } else { "read.revoke" },
                account,
                Some(connection),
            )
        }
    };
    tx.execute(
        "INSERT INTO admin_events(actor,action,subject,connection_id) VALUES($1,$2,$3,$4)",
        &[&actor, &action, &subject, &conn],
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn page() -> Html<&'static str> {
    Html(include_str!("../web/dist/index.html"))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    database_url: String,
    secret: String,
    origin: String,
    initial_admin_password: Option<String>,
}
fn load_config() -> std::result::Result<Config, Box<dyn std::error::Error>> {
    let explicit = env::var_os("DBW_CONFIG");
    let path = explicit
        .clone()
        .unwrap_or_else(|| "config/local.json".into());
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if explicit.is_none() && e.kind() == std::io::ErrorKind::NotFound => Ok(Config {
            database_url: env::var("DATABASE_URL")?,
            secret: env::var("DBW_SECRET")?,
            origin: env::var("DBW_ORIGIN")?,
            initial_admin_password: env::var("DBW_ADMIN_PASSWORD").ok(),
        }),
        Err(e) => Err(e.into()),
    }
}
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let cfg = config.database_url.parse()?;
    let pool = Pool::builder(Manager::new(cfg, NoTls))
        .max_size(12)
        .build()?;
    let key = config.secret;
    if key.len() < 32 {
        return Err("DBW_SECRET requires at least 32 bytes".into());
    }
    let origin = config.origin;
    if !(origin.starts_with("https://")
        || origin == "http://localhost:8080"
        || origin == "http://127.0.0.1:8080")
    {
        return Err("configure HTTPS origin or local development origin".into());
    }
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    tx.execute("SELECT pg_advisory_xact_lock(146421776)", &[])
        .await?;
    tx.batch_execute(include_str!("../migrations/001.sql"))
        .await?;
    tx.batch_execute(include_str!("../migrations/002.sql"))
        .await?;
    tx.batch_execute(include_str!("../migrations/003.sql"))
        .await?;
    if !tx
        .query_one("SELECT EXISTS(SELECT 1 FROM accounts WHERE admin)", &[])
        .await?
        .get::<_, bool>(0)
    {
        let pw = config
            .initial_admin_password
            .ok_or("initial_admin_password required for first startup")?;
        if !password_ok(&pw) {
            return Err("initial admin password requires 12–72 bytes".into());
        }
        tx.execute("INSERT INTO accounts(name,password_hash,admin) VALUES('admin',crypt($1,gen_salt('bf',12)),true)",&[&pw]).await?;
    }
    tx.commit().await?;
    drop(db);
    let app = Router::new()
        .route("/", get(page))
        .route("/admin", get(page))
        .route("/admin/login", get(page))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/connections", get(catalog))
        .route("/api/admin/audit", get(audit::list))
        .route("/api/admin/probe", post(probe::probe))
        .route("/api/admin", get(overview).post(change))
        .with_state(App {
            pool,
            key: Arc::new(key),
            origin: Arc::new(origin),
        });
    axum::serve(tokio::net::TcpListener::bind("0.0.0.0:8080").await?, app).await?;
    Ok(())
}
