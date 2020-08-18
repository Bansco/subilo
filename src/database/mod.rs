use actix::prelude::*;
use rusqlite::{Connection, Result};
use std::fs;
use std::path::Path;
use std::process;

mod migrate {
    use refinery::embed_migrations;
    embed_migrations!("src/database");
}

pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    pub fn new(path: &str) -> Self {
        fs::create_dir_all(&path).expect("Failed to create database directory");

        let database_path_buf = Path::new(path).join("subilo-database.db");
        let database_path = match database_path_buf.to_str() {
            Some(path) => path,
            None => {
                eprintln!(
                    "Failed to create database connection path {} + /subilo-database.db",
                    path
                );
                process::exit(1);
            }
        };

        let connection =
            Connection::open(database_path).expect("Failed to connect to the database");

        Self { connection }
    }
}

impl Actor for Database {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("Connected to the database");
        debug!("Running database migrations");
        migrate::migrations::runner()
            .run(&mut self.connection)
            .expect("Failed to run database migrations");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("Disconnected from database");
    }
}

#[derive(Message)]
#[rtype(result = "Result<usize>")]
pub struct Execute {
    pub query: String,
    pub params: Vec<String>,
}

impl Handler<Execute> for Database {
    type Result = Result<usize>;

    fn handle(&mut self, execute: Execute, _ctx: &mut Context<Self>) -> Self::Result {
        self.connection
            .execute(execute.query.as_str(), execute.params)
    }
}

#[derive(Message)]
#[rtype(result = "Result<Vec<T>, rusqlite::Error>")]
pub struct Query<T, F>
where
    T: 'static,
    F: FnMut(&rusqlite::Row<'_>) -> Result<T>,
{
    pub query: String,
    pub params: Vec<String>,
    pub map_result: F,
}

impl<T, F> Handler<Query<T, F>> for Database
where
    T: 'static,
    F: FnMut(&rusqlite::Row<'_>) -> Result<T>,
{
    type Result = Result<Vec<T>, rusqlite::Error>;

    fn handle(&mut self, query: Query<T, F>, _ctx: &mut Context<Self>) -> Self::Result {
        let result: Result<Vec<T>> = self
            .connection
            .prepare(query.query.as_str())?
            .query_map(query.params, query.map_result)?
            .collect();

        result
    }
}
