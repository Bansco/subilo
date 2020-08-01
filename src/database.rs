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
