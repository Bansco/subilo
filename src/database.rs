use crate::job;
use actix::prelude::*;
use rusqlite::NO_PARAMS;
use rusqlite::{Connection, Result};

pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    pub fn new(path: &str) -> Self {
        let connection = Connection::open(path).expect("Failed to connecet to the database");
        Self { connection }
    }

    fn create_tables(&self) -> Result<usize> {
        self.connection
            .execute(job::CREATE_JOB_TABLE_QUERY, NO_PARAMS)
    }
}

impl Actor for Database {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("Connected to database");
        self.create_tables().unwrap();
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("Disconnected from database");
    }
}


// TODO: Implement rusqlite into SubiloError 
#[derive(Message)]
#[rtype(result = "Result<usize>")]
pub struct Execute {
    pub query: String,
    pub params: Vec<String>,
}

impl Handler<Execute> for Database {
    type Result = Result<usize>;

    fn handle(&mut self, execute: Execute, _ctx: &mut Context<Self>) -> Result<usize> {
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
