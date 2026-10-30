//! Anonymous event log in SQLite with group commit: handlers never touch the database, they send
//! events to one writer thread, which batches them into a single transaction every 50 ms or
//! 256 events, whichever comes first. Stats are read through a second connection (WAL allows
//! reads while the writer commits).

use std::{
    path::Path,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};
use serde_json::Value;

const MAX_BATCH: usize = 256;
const MAX_WAIT: Duration = Duration::from_millis(50);

struct LogEvent {
    ts: i64,
    scenario_id: String,
    kind: String,
    detail: String,
}

enum Msg {
    Event(LogEvent),
    /// Commit whatever is pending, then signal back (used before reading stats).
    Flush(mpsc::Sender<()>),
}

#[derive(Default)]
pub struct WriterStats {
    pub events: AtomicU64,
    pub commits: AtomicU64,
}

#[derive(Clone)]
pub struct EventLog {
    tx: mpsc::Sender<Msg>,
    reader: Arc<Mutex<Connection>>,
    pub stats: Arc<WriterStats>,
}

impl EventLog {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let writer = Connection::open(path)?;
        writer.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS events (
                 id          INTEGER PRIMARY KEY,
                 ts          INTEGER NOT NULL,
                 scenario_id TEXT NOT NULL,
                 kind        TEXT NOT NULL,
                 detail      TEXT NOT NULL DEFAULT '{}'
             );
             CREATE INDEX IF NOT EXISTS events_scenario ON events (scenario_id, kind);",
        )?;
        let reader = Connection::open(path)?;
        let stats = Arc::new(WriterStats::default());
        let (tx, rx) = mpsc::channel();
        let writer_stats = stats.clone();
        thread::Builder::new()
            .name("event-writer".into())
            .spawn(move || writer_loop(writer, rx, &writer_stats))
            .expect("spawn event writer");
        Ok(Self {
            tx,
            reader: Arc::new(Mutex::new(reader)),
            stats,
        })
    }

    /// Non-blocking: queues the event for the writer thread.
    pub fn record(&self, scenario_id: &str, kind: &str, detail: &Value) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let _ = self.tx.send(Msg::Event(LogEvent {
            ts,
            scenario_id: scenario_id.to_owned(),
            kind: kind.to_owned(),
            detail: detail.to_string(),
        }));
    }

    /// Blocks until everything recorded so far is committed.
    pub fn flush(&self) {
        let (done, wait) = mpsc::channel();
        if self.tx.send(Msg::Flush(done)).is_ok() {
            let _ = wait.recv();
        }
    }

    pub fn reader(&self) -> MutexGuard<'_, Connection> {
        self.reader.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn writer_loop(mut conn: Connection, rx: mpsc::Receiver<Msg>, stats: &WriterStats) {
    let mut batch = Vec::with_capacity(MAX_BATCH);
    let mut flushes = Vec::new();
    while let Ok(first) = rx.recv() {
        let deadline = Instant::now() + MAX_WAIT;
        let mut next = Some(first);
        while let Some(msg) = next.take() {
            match msg {
                Msg::Event(e) => batch.push(e),
                Msg::Flush(done) => {
                    flushes.push(done);
                    break;
                }
            }
            if batch.len() >= MAX_BATCH {
                break;
            }
            next = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .ok();
        }
        if !batch.is_empty() {
            match commit(&mut conn, &batch) {
                Ok(()) => {
                    stats
                        .events
                        .fetch_add(batch.len() as u64, Ordering::Relaxed);
                    stats.commits.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => tracing::error!(error = %e, dropped = batch.len(), "event commit failed"),
            }
            batch.clear();
        }
        for done in flushes.drain(..) {
            let _ = done.send(());
        }
    }
}

fn commit(conn: &mut Connection, batch: &[LogEvent]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    {
        let mut insert = tx.prepare_cached(
            "INSERT INTO events (ts, scenario_id, kind, detail) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for e in batch {
            insert.execute(params![e.ts, e.scenario_id, e.kind, e.detail])?;
        }
    }
    tx.commit()
}

/// Per-scenario counters, aggregated in SQL.
pub struct ScenarioCounts {
    pub scenario_id: String,
    pub opens: u64,
    pub hovered_link: u64,
    pub clicked_link: u64,
    pub decisions: u64,
    pub correct: u64,
    pub fell_for_it: u64,
}

pub fn scenario_counts(conn: &Connection) -> rusqlite::Result<Vec<ScenarioCounts>> {
    let mut q = conn.prepare_cached(
        "SELECT scenario_id,
                SUM(kind = 'open'),
                SUM(kind = 'hover_link'),
                SUM(kind = 'click_link'),
                SUM(kind = 'decision'),
                SUM(kind = 'decision' AND json_extract(detail, '$.correct') = 1),
                SUM(kind = 'decision' AND json_extract(detail, '$.action') = 'fell_for_it')
         FROM events GROUP BY scenario_id",
    )?;
    q.query_map([], |r| {
        Ok(ScenarioCounts {
            scenario_id: r.get(0)?,
            opens: r.get::<_, Option<i64>>(1)?.unwrap_or(0) as u64,
            hovered_link: r.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
            clicked_link: r.get::<_, Option<i64>>(3)?.unwrap_or(0) as u64,
            decisions: r.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
            correct: r.get::<_, Option<i64>>(5)?.unwrap_or(0) as u64,
            fell_for_it: r.get::<_, Option<i64>>(6)?.unwrap_or(0) as u64,
        })
    })?
    .collect()
}
