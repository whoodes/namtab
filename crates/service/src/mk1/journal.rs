//! Durable reservation metadata only. Payloads, argv and their hashes never enter this file.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

const LIMIT: u64 = 32 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    marker: String,
    epoch: String,
    endpoint: String,
    id: u64,
    operation: String,
    state: String,
    delivered_bytes: usize,
}
pub struct Journal {
    file: File,
    epoch: String,
    size: u64,
}
impl Journal {
    pub fn open(path: &Path, epoch: String, uid: u32) -> io::Result<Self> {
        let metadata = std::fs::symlink_metadata(path).ok();
        let mut file = if let Some(metadata) = &metadata {
            if !metadata.is_file()
                || metadata.uid() != uid
                || metadata.mode() & 0o777 != 0o600
                || metadata.nlink() != 1
                || metadata.len() > LIMIT
            {
                return Err(io::Error::other("unsafe or full journal"));
            }
            let file = OpenOptions::new().read(true).append(true).open(path)?;
            let opened = file.metadata()?;
            if opened.ino() != metadata.ino() || opened.dev() != metadata.dev() {
                return Err(io::Error::other("journal replaced during open"));
            }
            file
        } else {
            OpenOptions::new()
                .read(true)
                .append(true)
                .create_new(true)
                .mode(0o600)
                .open(path)?
        };
        let mut prior = BTreeMap::new();
        if metadata.is_some() {
            let mut body = String::new();
            (&mut file).take(LIMIT + 1).read_to_string(&mut body)?;
            if body.is_empty() || body.len() as u64 > LIMIT || !body.ends_with('\n') {
                return Err(io::Error::other("incomplete or oversized journal"));
            }
            for line in body.lines() {
                let record: Record =
                    serde_json::from_str(line).map_err(|_| io::Error::other("unowned journal"))?;
                if record.marker != "namtab.mk1-journal/1"
                    || !matches!(
                        record.state.as_str(),
                        "epoch" | "reserved" | "rejected" | "applied" | "partial" | "unknown"
                    )
                {
                    return Err(io::Error::other("unowned journal"));
                }
                prior.insert(
                    (record.epoch.clone(), record.endpoint.clone(), record.id),
                    record,
                );
            }
        }
        let size = file.metadata()?.len();
        let mut journal = Self { file, epoch, size };
        for record in prior.values().filter(|record| record.state == "reserved") {
            journal.write(Record {
                marker: "namtab.mk1-journal/1".into(),
                epoch: record.epoch.clone(),
                endpoint: record.endpoint.clone(),
                id: record.id,
                operation: record.operation.clone(),
                state: "unknown".into(),
                delivered_bytes: 0,
            })?;
        }
        journal.record("system", 0, "daemon", "epoch", 0)?;
        File::open(
            path.parent()
                .ok_or_else(|| io::Error::other("journal parent"))?,
        )?
        .sync_all()?;
        Ok(journal)
    }
    fn write(&mut self, record: Record) -> io::Result<()> {
        let mut bytes = serde_json::to_vec(&record).map_err(io::Error::other)?;
        bytes.push(b'\n');
        if self.size + bytes.len() as u64 > LIMIT {
            return Err(io::Error::other("journal capacity"));
        }
        self.file.write_all(&bytes)?;
        self.file.sync_all()?;
        self.size += bytes.len() as u64;
        Ok(())
    }
    pub fn record(
        &mut self,
        endpoint: &str,
        id: u64,
        operation: &str,
        state: &str,
        delivered_bytes: usize,
    ) -> io::Result<()> {
        self.write(Record {
            marker: "namtab.mk1-journal/1".into(),
            epoch: self.epoch.clone(),
            endpoint: endpoint.into(),
            id,
            operation: operation.into(),
            state: state.into(),
            delivered_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn metadata_reservation_reconciles_unknown_without_payloads_and_refuses_unowned_file() {
        let directory =
            std::env::temp_dir().join(format!("mk1-journal-{}", super::super::token().unwrap()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("owned");
        let uid = project_x_platform::local_process::effective_uid();
        {
            let mut journal = Journal::open(&path, "first".into(), uid).unwrap();
            journal.record("e1", 1, "input", "reserved", 0).unwrap();
        }
        {
            let _journal = Journal::open(&path, "second".into(), uid).unwrap();
        }
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(
            body.lines()
                .map(|line| serde_json::from_str::<Record>(line).unwrap())
                .any(|record| record.epoch == "first"
                    && record.id == 1
                    && record.state == "unknown")
        );
        let other = directory.join("unrelated");
        std::fs::write(&other, "leave untouched\n").unwrap();
        std::fs::set_permissions(&other, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(Journal::open(&other, "third".into(), uid).is_err());
        assert_eq!(
            std::fs::read_to_string(&other).unwrap(),
            "leave untouched\n"
        );
        let mut journal = Journal::open(&path, "third".into(), uid).unwrap();
        journal.size = LIMIT;
        assert!(journal.record("e1", 2, "run", "reserved", 0).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
