use app_core::{
    Conversation, ConversationEditedFiles, ConversationId, ConversationStatus,
    ConversationWorkspaceState, EffectId, Message, MessageId, Project, ProjectId,
    SessionBaseRevision,
};
use crossbeam_channel::{unbounded, Sender};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::thread;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct Db {
    path: PathBuf,
}

impl Db {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        apply_pragmas(&conn)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { path })
    }

    pub fn connect(&self) -> Result<Connection, StorageError> {
        let conn = Connection::open(&self.path)?;
        apply_pragmas(&conn)?;
        Ok(conn)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_projects(&self) -> Result<Vec<Project>, StorageError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare_cached(
            "select id,name,path,created_at,updated_at from projects order by updated_at desc",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: ProjectId::from_string(row.get::<_, String>(0)?),
                name: row.get(1)?,
                path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn upsert_project(&self, project: &Project) -> Result<(), StorageError> {
        let conn = self.connect()?;
        conn.execute(
            "insert into projects(id,name,path,created_at,updated_at) values(?1,?2,?3,?4,?5)
             on conflict(id) do update set name=excluded.name,path=excluded.path,updated_at=excluded.updated_at",
            params![project.id.as_str(), project.name, project.path, project.created_at, project.updated_at],
        )?;
        Ok(())
    }

    pub fn upsert_conversation(&self, c: &Conversation) -> Result<(), StorageError> {
        let conn = self.connect()?;
        conn.execute(
            "insert into conversations(id,cursor_session_id,project_id,title,summary,last_message_preview,status,message_count,created_at,updated_at,last_opened_at)
             values(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
             on conflict(id) do update set cursor_session_id=excluded.cursor_session_id, project_id=excluded.project_id,
             title=excluded.title, summary=excluded.summary, last_message_preview=excluded.last_message_preview,
             status=excluded.status, message_count=excluded.message_count, updated_at=excluded.updated_at, last_opened_at=excluded.last_opened_at",
            params![
                c.id.as_str(), c.cursor_session_id, c.project_id.as_str(), c.title, c.summary,
                c.last_message_preview, format!("{:?}", c.status), c.message_count, c.created_at, c.updated_at, c.last_opened_at
            ],
        )?;
        Ok(())
    }

    pub fn load_conversations(&self, limit: usize) -> Result<Vec<Conversation>, StorageError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare_cached(
            "select id,cursor_session_id,project_id,title,summary,last_message_preview,status,message_count,created_at,updated_at,last_opened_at
             from conversations order by updated_at desc limit ?1",
        )?;
        let rows = stmt.query_map([limit as i64], Self::row_to_conversation)?;
        Self::map_conversation_rows(rows)
    }

    pub fn load_conversations_for_project(
        &self,
        project_id: &ProjectId,
        limit: usize,
    ) -> Result<Vec<Conversation>, StorageError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare_cached(
            "select id,cursor_session_id,project_id,title,summary,last_message_preview,status,message_count,created_at,updated_at,last_opened_at
             from conversations where project_id = ?1 order by updated_at desc limit ?2",
        )?;
        let rows = stmt.query_map(params![project_id.as_str(), limit as i64], Self::row_to_conversation)?;
        Self::map_conversation_rows(rows)
    }

    fn row_to_conversation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Conversation> {
        Ok(Conversation {
            id: ConversationId::from_string(row.get::<_, String>(0)?),
            cursor_session_id: row.get(1)?,
            project_id: ProjectId::from_string(row.get::<_, String>(2)?),
            title: row.get(3)?,
            summary: row.get(4)?,
            last_message_preview: row.get(5)?,
            status: parse_status(&row.get::<_, String>(6)?),
            message_count: row.get::<_, i64>(7)? as u32,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            last_opened_at: row.get(10)?,
        })
    }

    fn map_conversation_rows(
        rows: impl Iterator<Item = rusqlite::Result<Conversation>>,
    ) -> Result<Vec<Conversation>, StorageError> {
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn delete_conversation(&self, id: &ConversationId) -> Result<(), StorageError> {
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        tx.execute(
            "delete from messages where conversation_id = ?1",
            [id.as_str()],
        )?;
        tx.execute(
            "delete from acp_events where conversation_id = ?1",
            [id.as_str()],
        )?;
        tx.execute(
            "delete from conversation_session_bases where conversation_id = ?1",
            [id.as_str()],
        )?;
        tx.execute(
            "delete from conversation_workspace where conversation_id = ?1",
            [id.as_str()],
        )?;
        tx.execute(
            "delete from conversation_edited_files where conversation_id = ?1",
            [id.as_str()],
        )?;
        tx.execute("delete from conversations where id = ?1", [id.as_str()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_conversation(
        &self,
        id: &ConversationId,
    ) -> Result<Option<Conversation>, StorageError> {
        let conn = self.connect()?;
        conn.query_row(
            "select id,cursor_session_id,project_id,title,summary,last_message_preview,status,message_count,created_at,updated_at,last_opened_at from conversations where id=?1",
            [id.as_str()],
            |row| Ok(Conversation {
                id: ConversationId::from_string(row.get::<_, String>(0)?),
                cursor_session_id: row.get(1)?,
                project_id: ProjectId::from_string(row.get::<_, String>(2)?),
                title: row.get(3)?,
                summary: row.get(4)?,
                last_message_preview: row.get(5)?,
                status: parse_status(&row.get::<_, String>(6)?),
                message_count: row.get::<_, i64>(7)? as u32,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                last_opened_at: row.get(10)?,
            }),
        ).optional().map_err(StorageError::from)
    }

    pub fn insert_messages(&self, messages: &[Message]) -> Result<(), StorageError> {
        if messages.is_empty() {
            return Ok(());
        }
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "insert into messages(id,conversation_id,role,kind,text,ordinal,created_at,updated_at)
                 values(?1,?2,?3,?4,?5,?6,?7,?8)
                 on conflict(id) do update set text=excluded.text,kind=excluded.kind,updated_at=excluded.updated_at",
            )?;
            let mut fts = tx.prepare_cached(
                "insert or replace into messages_fts(rowid,conversation_id,title,text) values((select rowid from messages where id=?1),?2,'',?3)",
            )?;
            for m in messages {
                stmt.execute(params![
                    m.id.as_str(),
                    m.conversation_id.as_str(),
                    m.role,
                    m.kind,
                    m.text,
                    m.ordinal,
                    m.created_at,
                    m.updated_at
                ])?;
                let _ = fts.execute(params![m.id.as_str(), m.conversation_id.as_str(), m.text]);
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_recent_messages(
        &self,
        conversation_id: &ConversationId,
        limit: usize,
    ) -> Result<Vec<Message>, StorageError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare_cached(
            "select id,conversation_id,role,kind,coalesce(text,''),ordinal,created_at,updated_at
             from messages where conversation_id=?1 order by ordinal desc limit ?2",
        )?;
        let mut rows = stmt
            .query_map(params![conversation_id.as_str(), limit as i64], |row| {
                Ok(Message {
                    id: MessageId::from_string(row.get::<_, String>(0)?),
                    conversation_id: ConversationId::from_string(row.get::<_, String>(1)?),
                    role: row.get(2)?,
                    kind: row.get(3)?,
                    text: row.get(4)?,
                    ordinal: row.get::<_, i64>(5)? as u32,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.reverse();
        Ok(rows)
    }

    pub fn append_acp_event(
        &self,
        conversation_id: &ConversationId,
        direction: &str,
        method: Option<&str>,
        raw_json: &Value,
    ) -> Result<(), StorageError> {
        let conn = self.connect()?;
        conn.execute(
            "insert into acp_events(conversation_id,ts,direction,method,raw_json) values(?1,strftime('%s','now')*1000,?2,?3,?4)",
            params![conversation_id.as_str(), direction, method, serde_json::to_string(raw_json)?],
        )?;
        Ok(())
    }

    pub fn search_messages(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, StorageError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare_cached(
            "select conversation_id, snippet(messages_fts, 2, '[', ']', '...', 12), rank
             from messages_fts where messages_fts match ?1 order by rank limit ?2",
        )?;
        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Ok(SearchHit {
                conversation_id: ConversationId::from_string(row.get::<_, String>(0)?),
                snippet: row.get(1)?,
                rank: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn load_session_bases(&self) -> Result<Vec<SessionBaseRevision>, StorageError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare_cached(
            "select conversation_id,project_id,revision,branch,captured_at_ms
             from conversation_session_bases",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionBaseRevision {
                conversation_id: ConversationId::from_string(row.get::<_, String>(0)?),
                project_id: ProjectId::from_string(row.get::<_, String>(1)?),
                revision: row.get(2)?,
                branch: row.get(3)?,
                captured_at_ms: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn upsert_session_base(&self, session: &SessionBaseRevision) -> Result<(), StorageError> {
        let conn = self.connect()?;
        conn.execute(
            "insert into conversation_session_bases(conversation_id,project_id,revision,branch,captured_at_ms)
             values(?1,?2,?3,?4,?5)
             on conflict(conversation_id) do update set
               project_id=excluded.project_id,
               revision=excluded.revision,
               branch=excluded.branch,
               captured_at_ms=excluded.captured_at_ms",
            params![
                session.conversation_id.as_str(),
                session.project_id.as_str(),
                session.revision,
                session.branch,
                session.captured_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn load_conversation_workspaces(
        &self,
    ) -> Result<Vec<(ConversationId, ConversationWorkspaceState)>, StorageError> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare_cached("select conversation_id,state_json from conversation_workspace")?;
        let rows = stmt.query_map([], |row| {
            let conversation_id = ConversationId::from_string(row.get::<_, String>(0)?);
            let state_json: String = row.get(1)?;
            let workspace: ConversationWorkspaceState = serde_json::from_str(&state_json)
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok((conversation_id, workspace))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn upsert_conversation_workspace(
        &self,
        conversation_id: &ConversationId,
        workspace: &ConversationWorkspaceState,
    ) -> Result<(), StorageError> {
        let conn = self.connect()?;
        conn.execute(
            "insert into conversation_workspace(conversation_id,state_json,updated_at)
             values(?1,?2,strftime('%s','now')*1000)
             on conflict(conversation_id) do update set
               state_json=excluded.state_json,
               updated_at=excluded.updated_at",
            params![
                conversation_id.as_str(),
                serde_json::to_string(workspace)?,
            ],
        )?;
        Ok(())
    }

    pub fn load_conversation_edited_files(
        &self,
    ) -> Result<Vec<(ConversationId, ConversationEditedFiles)>, StorageError> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare_cached("select conversation_id,state_json from conversation_edited_files")?;
        let rows = stmt.query_map([], |row| {
            let conversation_id = ConversationId::from_string(row.get::<_, String>(0)?);
            let state_json: String = row.get(1)?;
            let edited_files: ConversationEditedFiles = serde_json::from_str(&state_json)
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok((conversation_id, edited_files))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn upsert_conversation_edited_files(
        &self,
        conversation_id: &ConversationId,
        edited_files: &ConversationEditedFiles,
    ) -> Result<(), StorageError> {
        let conn = self.connect()?;
        conn.execute(
            "insert into conversation_edited_files(conversation_id,state_json,updated_at)
             values(?1,?2,strftime('%s','now')*1000)
             on conflict(conversation_id) do update set
               state_json=excluded.state_json,
               updated_at=excluded.updated_at",
            params![
                conversation_id.as_str(),
                serde_json::to_string(edited_files)?,
            ],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub conversation_id: ConversationId,
    pub snippet: String,
    pub rank: i64,
}

#[derive(Debug, Clone)]
pub enum StorageWrite {
    UpsertProject {
        effect_id: EffectId,
        project: Project,
    },
    UpsertConversation {
        effect_id: EffectId,
        conversation: Conversation,
    },
    DeleteConversation {
        effect_id: EffectId,
        conversation_id: ConversationId,
    },
    InsertMessages {
        effect_id: EffectId,
        conversation_id: ConversationId,
        messages: Vec<Message>,
    },
    AppendAcpEvent {
        effect_id: EffectId,
        conversation_id: ConversationId,
        direction: String,
        method: Option<String>,
        raw_json: Value,
    },
    UpsertSessionBase {
        effect_id: EffectId,
        session: SessionBaseRevision,
    },
    UpsertConversationWorkspace {
        effect_id: EffectId,
        conversation_id: ConversationId,
        workspace: ConversationWorkspaceState,
    },
    UpsertConversationEditedFiles {
        effect_id: EffectId,
        conversation_id: ConversationId,
        edited_files: ConversationEditedFiles,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum StorageWriteResult {
    Completed { effect_id: EffectId },
    Failed { effect_id: EffectId, error: String },
}

pub struct StorageWriter {
    tx: Sender<StorageWrite>,
}

impl StorageWriter {
    pub fn start(db: Db, result_tx: Sender<StorageWriteResult>) -> Self {
        let (tx, rx) = unbounded::<StorageWrite>();
        thread::spawn(move || {
            while let Ok(write) = rx.recv() {
                match write {
                    StorageWrite::Shutdown => break,
                    StorageWrite::UpsertProject { effect_id, project } => {
                        send_result(effect_id, db.upsert_project(&project), &result_tx)
                    }
                    StorageWrite::UpsertConversation {
                        effect_id,
                        conversation,
                    } => send_result(effect_id, db.upsert_conversation(&conversation), &result_tx),
                    StorageWrite::DeleteConversation {
                        effect_id,
                        conversation_id,
                    } => send_result(
                        effect_id,
                        db.delete_conversation(&conversation_id),
                        &result_tx,
                    ),
                    StorageWrite::InsertMessages {
                        effect_id,
                        messages,
                        ..
                    } => send_result(effect_id, db.insert_messages(&messages), &result_tx),
                    StorageWrite::AppendAcpEvent {
                        effect_id,
                        conversation_id,
                        direction,
                        method,
                        raw_json,
                    } => send_result(
                        effect_id,
                        db.append_acp_event(
                            &conversation_id,
                            &direction,
                            method.as_deref(),
                            &raw_json,
                        ),
                        &result_tx,
                    ),
                    StorageWrite::UpsertSessionBase { effect_id, session } => {
                        send_result(effect_id, db.upsert_session_base(&session), &result_tx)
                    }
                    StorageWrite::UpsertConversationWorkspace {
                        effect_id,
                        conversation_id,
                        workspace,
                    } => send_result(
                        effect_id,
                        db.upsert_conversation_workspace(&conversation_id, &workspace),
                        &result_tx,
                    ),
                    StorageWrite::UpsertConversationEditedFiles {
                        effect_id,
                        conversation_id,
                        edited_files,
                    } => send_result(
                        effect_id,
                        db.upsert_conversation_edited_files(&conversation_id, &edited_files),
                        &result_tx,
                    ),
                }
            }
        });
        Self { tx }
    }

    pub fn enqueue(&self, write: StorageWrite) -> Result<(), StorageError> {
        self.tx.send(write).map_err(|e| {
            StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                e.to_string(),
            ))
        })
    }
}

fn send_result(
    effect_id: EffectId,
    result: Result<(), StorageError>,
    result_tx: &Sender<StorageWriteResult>,
) {
    let _ = match result {
        Ok(()) => result_tx.send(StorageWriteResult::Completed { effect_id }),
        Err(error) => result_tx.send(StorageWriteResult::Failed {
            effect_id,
            error: error.to_string(),
        }),
    };
}

fn apply_pragmas(conn: &Connection) -> Result<(), StorageError> {
    conn.pragma_update(None, "journal_mode", "wal")?;
    conn.pragma_update(None, "synchronous", "normal")?;
    conn.pragma_update(None, "temp_store", "memory")?;
    conn.pragma_update(None, "foreign_keys", "on")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "mmap_size", 268_435_456_i64)?;
    conn.pragma_update(None, "cache_size", -65_536_i64)?;
    Ok(())
}

fn parse_status(value: &str) -> ConversationStatus {
    match value {
        "Starting" => ConversationStatus::Starting,
        "Running" => ConversationStatus::Running,
        "WaitingForPermission" => ConversationStatus::WaitingForPermission,
        "Paused" => ConversationStatus::Paused,
        "Throttling" => ConversationStatus::Throttling,
        "Completed" => ConversationStatus::Completed,
        "Cancelled" => ConversationStatus::Cancelled,
        "Failed" => ConversationStatus::Failed,
        _ => ConversationStatus::Idle,
    }
}

const SCHEMA: &str = r#"
create table if not exists projects (
  id text primary key,
  name text not null,
  path text not null unique,
  created_at integer not null,
  updated_at integer not null
);

create table if not exists conversations (
  id text primary key,
  cursor_session_id text,
  project_id text not null,
  title text not null,
  summary text,
  last_message_preview text,
  status text not null,
  message_count integer not null default 0,
  created_at integer not null,
  updated_at integer not null,
  last_opened_at integer
);

create table if not exists messages (
  id text primary key,
  conversation_id text not null,
  role text not null,
  kind text not null,
  text text,
  ordinal integer not null,
  created_at integer not null,
  updated_at integer not null
);

create index if not exists messages_conversation_ordinal on messages(conversation_id, ordinal desc);
create index if not exists conversations_project_updated on conversations(project_id, updated_at desc);

create table if not exists rendered_blocks (
  id text primary key,
  conversation_id text not null,
  message_id text not null,
  block_kind text not null,
  content blob not null,
  content_hash text not null,
  height_estimate integer,
  created_at integer not null
);

create table if not exists acp_events (
  id integer primary key autoincrement,
  conversation_id text not null,
  ts integer not null,
  direction text not null,
  method text,
  raw_json text not null
);

create virtual table if not exists messages_fts using fts5(
  conversation_id,
  title,
  text,
  tokenize = 'unicode61'
);

create table if not exists conversation_session_bases (
  conversation_id text primary key,
  project_id text not null,
  revision text not null,
  branch text,
  captured_at_ms integer not null
);

create table if not exists conversation_workspace (
  conversation_id text primary key,
  state_json text not null,
  updated_at integer not null
);

create table if not exists conversation_edited_files (
  conversation_id text primary key,
  state_json text not null,
  updated_at integer not null
);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use app_core::{
        ConversationEditedFiles, ConversationId, FileReviewView, ProjectId, RightPaneMode,
    };

    #[test]
    fn session_base_and_workspace_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "mac-agent-cockpit-storage-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");
        let db = Db::open(&db_path).unwrap();

        let conversation_id = ConversationId::new();
        let session = SessionBaseRevision {
            conversation_id: conversation_id.clone(),
            project_id: ProjectId::new(),
            revision: "abc123".into(),
            branch: Some("main".into()),
            captured_at_ms: 1_700_000_000_000,
        };
        db.upsert_session_base(&session).unwrap();

        let workspace = ConversationWorkspaceState {
            right_pane_mode: RightPaneMode::Browser,
            selected_path: Some("src/main.rs".into()),
            selected_review_view: FileReviewView::InlineChanges,
            expanded_directories: vec![".".into(), "src".into()],
            browser_url: Some("https://example.com".into()),
        };
        db.upsert_conversation_workspace(&conversation_id, &workspace)
            .unwrap();

        let edited_files = ConversationEditedFiles {
            count: 2,
            paths: vec!["src/lib.rs".into(), "src/main.rs".into()],
        };
        db.upsert_conversation_edited_files(&conversation_id, &edited_files)
            .unwrap();

        let loaded_sessions = db.load_session_bases().unwrap();
        assert_eq!(loaded_sessions, vec![session]);

        let loaded_workspaces = db.load_conversation_workspaces().unwrap();
        assert_eq!(loaded_workspaces, vec![(conversation_id.clone(), workspace)]);

        let loaded_edited_files = db.load_conversation_edited_files().unwrap();
        assert_eq!(
            loaded_edited_files,
            vec![(conversation_id.clone(), edited_files)]
        );

        db.delete_conversation(&conversation_id).unwrap();
        assert!(db.load_session_bases().unwrap().is_empty());
        assert!(db.load_conversation_workspaces().unwrap().is_empty());
        assert!(db.load_conversation_edited_files().unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
