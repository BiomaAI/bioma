use crate::actor::SystemActorError;
use bon::builder;
use derive_more::Display;
use object_store::local::LocalFileSystem;
use serde::{Deserialize, Serialize};
use surrealdb::{
    engine::any::{Any, IntoEndpoint},
    opt::auth::Root,
    value::RecordId,
    Surreal,
};

#[macro_export]
macro_rules! dbg_export_db {
    ($engine:expr) => {{
        let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let output_dir = workspace_root.join("output").join("debug");
        std::fs::create_dir_all(&output_dir).unwrap();

        let file_name = format!("dbg_{}_{}", file!().replace("/", "_").replace(".", "_"), line!());
        let file_path = output_dir.join(format!("{}.surql", file_name));

        $engine.db().export(file_path.to_str().unwrap()).await.unwrap();
    }};
}

// Record struct for database entries
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Display)]
pub struct Record {
    pub id: RecordId,
}

/// Configuration options for the Engine.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineOptions {
    /// The namespace to use in the database.
    pub namespace: String,
    /// The name of the database to connect to.
    pub database: String,
    /// The username for database authentication.
    pub username: String,
    /// The password for database authentication.
    pub password: String,
    /// The local file system path for object store.
    pub local_store: std::path::PathBuf,
}

impl Default for EngineOptions {
    fn default() -> Self {
        let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let local_store = workspace_root.join("output").join("store");
        std::fs::create_dir_all(&local_store).unwrap();
        EngineOptions {
            namespace: "dev".to_string(),
            database: "bioma".to_string(),
            username: "root".to_string(),
            password: "root".to_string(),
            local_store,
        }
    }
}

/// The engine is the main entry point for the Actor framework.
/// Responsible for creating and managing the database connection.
#[derive(Clone, Debug)]
pub struct Engine {
    db: Box<Surreal<Any>>,
    options: EngineOptions,
}

impl Engine {
    pub fn db(&self) -> &Surreal<Any> {
        &self.db
    }

    pub async fn connect(address: impl IntoEndpoint, options: EngineOptions) -> Result<Engine, SystemActorError> {
        let db: Surreal<Any> = Surreal::init();
        db.connect(address).await?;
        db.signin(Root { username: &options.username, password: &options.password }).await?;
        db.use_ns(&options.namespace).use_db(&options.database).await?;
        Engine::define(&db).await?;
        Ok(Engine { db: Box::new(db), options })
    }

    pub async fn test() -> Result<Engine, SystemActorError> {
        let options = EngineOptions::default();
        let db: Surreal<Any> = Surreal::init();
        db.connect("memory").await?;
        db.use_ns(&options.namespace).use_db(&options.database).await?;
        Engine::define(&db).await?;
        Ok(Engine { db: Box::new(db), options })
    }

    pub async fn health(&self) -> bool {
        self.db.health().await.is_ok()
    }

    async fn define(db: &Surreal<Any>) -> Result<(), SystemActorError> {
        // load the surreal definition file
        // let def = std::fs::read_to_string("assets/surreal/def.surql").unwrap();
        let def = include_str!("../sql/def.surql").parse::<String>().unwrap();
        db.query(&def).await?;
        Ok(())
    }

    pub fn local_store(&self) -> Result<LocalFileSystem, SystemActorError> {
        let store = LocalFileSystem::new_with_prefix(self.options.local_store.clone())?;
        Ok(store)
    }

    pub fn debug_output_dir(&self) -> Result<std::path::PathBuf, SystemActorError> {
        let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let output_dir = workspace_root.join("output").join("debug");
        std::fs::create_dir_all(&output_dir).unwrap();
        Ok(output_dir)
    }

    pub fn options(&self) -> &EngineOptions {
        &self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect() {
        let engine = Engine::test().await.unwrap();
        assert_eq!(engine.health().await, true);
    }
}
