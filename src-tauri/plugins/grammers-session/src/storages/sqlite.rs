// Copyright 2020 - developers of the `grammers` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::defs::{
    ChannelKind, ChannelState, DcOption, PeerAuth, PeerId, PeerInfo, PeerKind, UpdateState,
    UpdatesState,
};
use crate::{DEFAULT_DC, KNOWN_DC_OPTIONS, Session};
use std::path::Path;
use std::sync::Mutex;

const VERSION: i64 = 1;

struct Database(rusqlite::Connection);

/// SQLite-based storage backed by rusqlite.
pub struct SqliteSession {
    database: Mutex<rusqlite::Connection>,
}

#[repr(u8)]
enum PeerSubtype {
    UserSelf = 1,
    UserBot = 2,
    UserSelfBot = 3,
    Megagroup = 4,
    Broadcast = 8,
    Gigagroup = 12,
}

impl Database {
    fn init(&self) -> rusqlite::Result<()> {
        let user_version: i64 = self
            .0
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap_or(0);
        if user_version == VERSION {
            return Ok(());
        }

        if user_version == 0 {
            self.migrate_v0_to_v1()?;
        }
        self.0
            .execute_batch(&format!("PRAGMA user_version = {VERSION}"))?;
        Ok(())
    }

    fn migrate_v0_to_v1(&self) -> rusqlite::Result<()> {
        self.0.execute_batch(
            "BEGIN TRANSACTION;
             CREATE TABLE IF NOT EXISTS dc_home (
                 dc_id INTEGER NOT NULL,
                 PRIMARY KEY(dc_id));
             CREATE TABLE IF NOT EXISTS dc_option (
                 dc_id INTEGER NOT NULL,
                 ipv4 TEXT NOT NULL,
                 ipv6 TEXT NOT NULL,
                 auth_key BLOB,
                 PRIMARY KEY (dc_id));
             CREATE TABLE IF NOT EXISTS peer_info (
                 peer_id INTEGER NOT NULL,
                 hash INTEGER,
                 subtype INTEGER,
                 PRIMARY KEY (peer_id));
             CREATE TABLE IF NOT EXISTS update_state (
                 pts INTEGER NOT NULL,
                 qts INTEGER NOT NULL,
                 date INTEGER NOT NULL,
                 seq INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS channel_state (
                 peer_id INTEGER NOT NULL,
                 pts INTEGER NOT NULL,
                 PRIMARY KEY (peer_id));
             COMMIT;",
        )?;
        Ok(())
    }
}

impl SqliteSession {
    /// Open a connection to the SQLite database at `path`,
    /// creating one if it doesn't exist.
    pub fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        let db = Database(conn);
        db.init()?;
        Ok(SqliteSession {
            database: Mutex::new(db.0),
        })
    }
}

impl Session for SqliteSession {
    fn home_dc_id(&self) -> i32 {
        let db = self.database.lock().unwrap();
        db.query_row("SELECT dc_id FROM dc_home LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|id| id as i32)
        .unwrap_or(DEFAULT_DC)
    }

    fn set_home_dc_id(&self, dc_id: i32) {
        let db = self.database.lock().unwrap();
        let _ = db.execute_batch("BEGIN;");
        let _ = db.execute("DELETE FROM dc_home", []);
        let _ = db.execute(
            "INSERT INTO dc_home VALUES (?1)",
            rusqlite::params![dc_id as i64],
        );
        let _ = db.execute_batch("COMMIT;");
    }

    fn dc_option(&self, dc_id: i32) -> Option<DcOption> {
        let db = self.database.lock().unwrap();
        let res = db
            .query_row(
                "SELECT dc_id, ipv4, ipv6, auth_key FROM dc_option WHERE dc_id = ?1 LIMIT 1",
                rusqlite::params![dc_id as i64],
                |row| {
                    let id: i64 = row.get(0)?;
                    let ipv4_str: String = row.get(1)?;
                    let ipv6_str: String = row.get(2)?;
                    let auth_key: Option<Vec<u8>> = row.get(3)?;
                    Ok(DcOption {
                        id: id as _,
                        ipv4: ipv4_str.parse().unwrap(),
                        ipv6: ipv6_str.parse().unwrap(),
                        auth_key: auth_key.map(|k| k.try_into().unwrap()),
                    })
                },
            )
            .ok();

        res.or_else(|| {
            KNOWN_DC_OPTIONS
                .iter()
                .find(|dc_option| dc_option.id == dc_id)
                .cloned()
        })
    }

    fn set_dc_option(&self, dc_option: &DcOption) {
        let db = self.database.lock().unwrap();
        let _ = db.execute(
            "INSERT OR REPLACE INTO dc_option VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                dc_option.id as i64,
                dc_option.ipv4.to_string(),
                dc_option.ipv6.to_string(),
                dc_option.auth_key.as_ref().map(|k| k.as_slice()),
            ],
        );
    }

    fn peer(&self, peer: PeerId) -> Option<PeerInfo> {
        let db = self.database.lock().unwrap();
        let (query, param) = if peer.kind() == PeerKind::UserSelf {
            (
                "SELECT peer_id, hash, subtype FROM peer_info WHERE subtype & ?1 LIMIT 1",
                PeerSubtype::UserSelf as i64,
            )
        } else {
            (
                "SELECT peer_id, hash, subtype FROM peer_info WHERE peer_id = ?1 LIMIT 1",
                peer.bot_api_dialog_id(),
            )
        };

        db.query_row(query, rusqlite::params![param], |row| {
            let peer_id: i64 = row.get(0)?;
            let hash: Option<i64> = row.get(1)?;
            let subtype: Option<i64> = row.get(2)?;
            let s = subtype.map(|v| v as u8);
            Ok(match peer.kind() {
                PeerKind::User | PeerKind::UserSelf => PeerInfo::User {
                    id: PeerId::user(peer_id).bare_id(),
                    auth: hash.map(PeerAuth::from_hash),
                    bot: s.map(|v| v & PeerSubtype::UserBot as u8 != 0),
                    is_self: s.map(|v| v & PeerSubtype::UserSelf as u8 != 0),
                },
                PeerKind::Chat => PeerInfo::Chat { id: peer.bare_id() },
                PeerKind::Channel => PeerInfo::Channel {
                    id: peer.bare_id(),
                    auth: hash.map(PeerAuth::from_hash),
                    kind: s.and_then(|v| {
                        if (v & PeerSubtype::Gigagroup as u8) == PeerSubtype::Gigagroup as _ {
                            Some(ChannelKind::Gigagroup)
                        } else if v & PeerSubtype::Broadcast as u8 != 0 {
                            Some(ChannelKind::Broadcast)
                        } else if v & PeerSubtype::Megagroup as u8 != 0 {
                            Some(ChannelKind::Megagroup)
                        } else {
                            None
                        }
                    }),
                },
            })
        })
        .ok()
    }

    fn cache_peer(&self, peer: &PeerInfo) {
        let db = self.database.lock().unwrap();
        let hash = if peer.auth() != PeerAuth::default() {
            Some(peer.auth().hash())
        } else {
            None
        };
        let subtype = match peer {
            PeerInfo::User { bot, is_self, .. } => {
                match (bot.unwrap_or_default(), is_self.unwrap_or_default()) {
                    (true, true) => Some(PeerSubtype::UserSelfBot),
                    (true, false) => Some(PeerSubtype::UserBot),
                    (false, true) => Some(PeerSubtype::UserSelf),
                    (false, false) => None,
                }
            }
            PeerInfo::Chat { .. } => None,
            PeerInfo::Channel { kind, .. } => kind.map(|kind| match kind {
                ChannelKind::Megagroup => PeerSubtype::Megagroup,
                ChannelKind::Broadcast => PeerSubtype::Broadcast,
                ChannelKind::Gigagroup => PeerSubtype::Gigagroup,
            }),
        };
        let _ = db.execute(
            "INSERT OR REPLACE INTO peer_info VALUES (?1, ?2, ?3)",
            rusqlite::params![
                peer.id().bot_api_dialog_id(),
                hash,
                subtype.map(|s| s as i64),
            ],
        );
    }

    fn updates_state(&self) -> UpdatesState {
        let db = self.database.lock().unwrap();
        let mut state = db
            .query_row(
                "SELECT pts, qts, date, seq FROM update_state LIMIT 1",
                [],
                |row| {
                    Ok(UpdatesState {
                        pts: row.get::<_, i64>(0)? as _,
                        qts: row.get::<_, i64>(1)? as _,
                        date: row.get::<_, i64>(2)? as _,
                        seq: row.get::<_, i64>(3)? as _,
                        channels: Vec::new(),
                    })
                },
            )
            .unwrap_or_default();

        if let Ok(mut stmt) = db.prepare("SELECT peer_id, pts FROM channel_state") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(ChannelState {
                    id: row.get(0)?,
                    pts: row.get::<_, i64>(1)? as _,
                })
            }) {
                state.channels = rows.filter_map(Result::ok).collect();
            }
        }
        state
    }

    fn set_update_state(&self, update: UpdateState) {
        let db = self.database.lock().unwrap();
        let _ = db.execute_batch("BEGIN;");
        match update {
            UpdateState::All(updates_state) => {
                let _ = db.execute("DELETE FROM update_state", []);
                let _ = db.execute(
                    "INSERT INTO update_state VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        updates_state.pts as i64,
                        updates_state.qts as i64,
                        updates_state.date as i64,
                        updates_state.seq as i64,
                    ],
                );
                let _ = db.execute("DELETE FROM channel_state", []);
                for channel in updates_state.channels {
                    let _ = db.execute(
                        "INSERT INTO channel_state VALUES (?1, ?2)",
                        rusqlite::params![channel.id, channel.pts as i64],
                    );
                }
            }
            UpdateState::Primary { pts, date, seq } => {
                let exists: bool = db
                    .query_row("SELECT 1 FROM update_state LIMIT 1", [], |_| Ok(true))
                    .unwrap_or(false);
                if exists {
                    let _ = db.execute(
                        "UPDATE update_state SET pts = ?1, date = ?2, seq = ?3",
                        rusqlite::params![pts as i64, date as i64, seq as i64],
                    );
                } else {
                    let _ = db.execute(
                        "INSERT INTO update_state VALUES (?1, 0, ?2, ?3)",
                        rusqlite::params![pts as i64, date as i64, seq as i64],
                    );
                }
            }
            UpdateState::Secondary { qts } => {
                let exists: bool = db
                    .query_row("SELECT 1 FROM update_state LIMIT 1", [], |_| Ok(true))
                    .unwrap_or(false);
                if exists {
                    let _ = db.execute(
                        "UPDATE update_state SET qts = ?1",
                        rusqlite::params![qts as i64],
                    );
                } else {
                    let _ = db.execute(
                        "INSERT INTO update_state VALUES (0, ?1, 0, 0)",
                        rusqlite::params![qts as i64],
                    );
                }
            }
            UpdateState::Channel { id, pts } => {
                let _ = db.execute(
                    "INSERT OR REPLACE INTO channel_state VALUES (?1, ?2)",
                    rusqlite::params![id, pts as i64],
                );
            }
        }
        let _ = db.execute_batch("COMMIT;");
    }
}
