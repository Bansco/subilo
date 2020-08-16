pub const INSERT_JOB: &str = "
    INSERT INTO jobs (id, name, status, started_at)
    VALUES (?1, ?2, ?3, ?4)
";

pub const UPDATE_JOB: &str = "
    UPDATE jobs
    SET status = ?2, ended_at = ?3
    WHERE id = ?1
";

pub const GET_ALL_JOBS: &str = "
    SELECT id, name, status, started_at, ended_at
    FROM jobs
";

pub const GET_JOB_BY_ID: &str = "
    SELECT id, name, status, started_at, ended_at
    FROM jobs
    WHERE id = ?1
";
