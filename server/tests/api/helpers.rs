use std::path::Path;

use lux_server::{
    build_router,
    config::Config,
    store::{kv::KvStore, sqlite},
};
use tempfile::TempDir;
use tokio::net::TcpListener;

pub struct TestApp {
    pub address: String,
    pub client: reqwest::Client,
    /// Keeps the temporary database files alive for the test's duration.
    _dir: TempDir,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.address)
    }
}

/// Starts the app on `127.0.0.1:<random port>` against fresh stores, using
/// the development config.
pub async fn spawn_app() -> TestApp {
    let dir = tempfile::tempdir().unwrap();

    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/config_dev.yaml");
    let mut config = Config::load_with_overrides(&config_path, None, None).unwrap();
    config.sqlite.dsn = dir.path().join("test.db").to_str().unwrap().to_owned();
    config.kv.path = dir.path().join("test.redb").to_str().unwrap().to_owned();

    let db = sqlite::connect(&config.sqlite.dsn).await.unwrap();
    sqlite::migrate(&db).await.unwrap();
    let kv = KvStore::open(&config.kv.path).await.unwrap();

    let app = build_router(db, kv, &config);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestApp {
        address,
        client: reqwest::Client::new(),
        _dir: dir,
    }
}
