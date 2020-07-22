use actix::prelude::*;
use rusqlite::NO_PARAMS;
use rusqlite::{Connection, Result};

pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    pub fn new(path: &str) -> Self {
        let connection = Connection::open(path).unwrap();
        Self { connection }
    }
}

// Provide Actor implementation for Database
impl Actor for Database {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        let query = "CREATE TABLE IF NOT EXISTS jobs (id integer primary key, name text not null)";
        self.connection.execute(query, NO_PARAMS).unwrap();
    }

    fn stopped(&mut self, ctx: &mut Context<Self>) {
        debug!("Actor is stopped");
    }
}

/// Define handler for `Ping` message
impl Handler<Ping> for Database {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, msg: Ping, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Ping received");

        Ok(true)
    }
}

struct Ping;
impl Message for Ping {
    type Result = Result<bool, std::io::Error>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct NewJob {
    pub id: String,
    pub name: String,
}

// Simple message handler for NewJob message
impl Handler<NewJob> for Database {
    type Result = ();

    fn handle(&mut self, msg: NewJob, ctx: &mut Context<Self>) {
        let query = "INSERT INTO jobs (id, name) VALUES (?1, ?2)";
        match self.connection.execute(query, &[&msg.id, &msg.name]) {
            Ok(_) => debug!("Job saved in the database"),
            Err(err) => error!("Failed to save job in the database: {}", err),
        }
    }
}
