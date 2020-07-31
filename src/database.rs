use actix::prelude::*;
use rusqlite::NO_PARAMS;
use rusqlite::{params, Connection, Result};

pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    pub fn new(path: &str) -> Self {
        let connection = Connection::open(path).expect("Failed to connecet to the local database");
        Self { connection }
    }
}

impl Actor for Database {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        let query = "
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT
            )
        ";
        self.connection.execute(query, NO_PARAMS).unwrap();
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("Actor is stopped");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct NewJob {
    pub id: String,
    pub name: String,
    pub started_at: String,
}

impl Handler<NewJob> for Database {
    type Result = ();

    fn handle(&mut self, job: NewJob, _ctx: &mut Context<Self>) {
        let query = "
            INSERT INTO jobs (id, name, status, started_at)
            VALUES (?1, ?2, ?3, ?4)
        ";
        let params = params![job.id, job.name, "started".to_owned(), job.started_at];

        match self.connection.execute(query, params) {
            Ok(_) => debug!("Job inserted successfully"),
            Err(err) => error!("Failed to insert job {}", err),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateJob {
    pub id: String,
    pub status: String,
    pub ended_at: String,
}

impl Handler<UpdateJob> for Database {
    type Result = ();

    fn handle(&mut self, job: UpdateJob, _ctx: &mut Context<Self>) {
        let query = "
            UPDATE jobs
            SET status = ?2, ended_at = ?3
            WHERE id = ?1
        ";
        let params = params![job.id, job.status, job.ended_at];

        match self.connection.execute(query, params) {
            Ok(_) => debug!("Job updated successfully"),
            Err(err) => error!("Failed to update job {}", err),
        }
    }
}
