use std::path::Path;

/// Provides an interface to a least frecency-used cache.
/// Allows one to journal the usage of resources (identified by fingerprints)
/// on disk, and calculate which resource is most optimal to delete.
pub struct Journal {
    db: rusqlite::Connection,
    maximum_resources: usize,
}

impl Journal {
    pub fn new(path: impl AsRef<Path>, maximum_resources: usize) -> anyhow::Result<Self> {
        let mut db = Self {
            db: rusqlite::Connection::open(path)?,
            maximum_resources,
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&mut self) -> anyhow::Result<()> {
        self.db.execute(
            r#"
            CREATE TABLE IF NOT EXISTS resources (
                fingerprint VARCHAR PRIMARY KEY,
                last_used DATETIME NOT NULL
            )
            "#,
            (),
        )?;
        Ok(())
    }

    /// Records that a resource was used.
    /// If the number of resources used exceeds the maximum allocated amount
    /// this function will also return a fingerprint whose resource should be deleted.
    /// That fingerprint is determined by the least-frecent (recent + frequent) resource.
    pub fn record_usage(&self, fingerprint: &str) -> anyhow::Result<Vec<String>> {
        log::debug!("Recording usage of fingerprint: `{}`", fingerprint);
        let now = chrono::Utc::now();
        self.db.execute(
            r#"
            INSERT INTO resources(
                fingerprint,
                last_used
            ) VALUES (
                ?,
                ?
            ) ON CONFLICT(fingerprint)
            DO UPDATE SET last_used=?
            "#,
            (fingerprint, now, now),
        )?;

        let mut stmt = self.db.prepare(
            r#"
            SELECT *
            FROM (
                SELECT
                    fingerprint,
                    last_used,
                    ROW_NUMBER() OVER (ORDER BY last_used DESC) AS row_num
                FROM resources
            )
            WHERE row_num > ?
            ORDER BY last_used ASC
            "#,
        )?;
        let expired_resources: Vec<String> = stmt
            .query_map((self.maximum_resources,), |row| row.get(0))?
            .flatten()
            .collect();

        Ok(expired_resources)
    }

    /// Marks a particular resource as deleted.
    pub fn mark_deleted(&self, fingerprint: &str) -> anyhow::Result<()> {
        log::debug!("Marking fingerprint as deleted: `{}`", fingerprint);
        self.db.execute(
            "DELETE FROM resources WHERE fingerprint = ?",
            (fingerprint,),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    fn test_journal(maximum_resources: usize) -> anyhow::Result<(TempDir, Journal)> {
        let tempdir = TempDir::new("venvcache-journal-test")?;
        let path = tempdir.path().join("journal.db");
        Ok((tempdir, Journal::new(path, maximum_resources)?))
    }

    #[test]
    fn test_journal_migrateable() -> anyhow::Result<()> {
        test_journal(10)?;
        Ok(())
    }

    #[test]
    fn test_journal_record() -> anyhow::Result<()> {
        let (_tempdir, journal) = test_journal(10)?;
        let expired_resources = journal.record_usage("fingerprint")?;
        assert_eq!(expired_resources, Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn test_journal_eviction() -> anyhow::Result<()> {
        let (_tempdir, journal) = test_journal(1)?;
        let expired_resources1 = journal.record_usage("fingerprint1")?;
        assert_eq!(expired_resources1, Vec::<String>::new());

        let expired_resources2 = journal.record_usage("fingerprint2")?;
        assert_eq!(expired_resources2, vec!["fingerprint1"]);

        let expired_resources3 = journal.record_usage("fingerprint3")?;
        assert_eq!(expired_resources3, vec!["fingerprint1", "fingerprint2"]);

        Ok(())
    }

    #[test]
    fn test_journal_mark_deleted() -> anyhow::Result<()> {
        let (_tempdir, journal) = test_journal(1)?;
        let expired_resources1 = journal.record_usage("fingerprint1")?;
        assert_eq!(expired_resources1, Vec::<String>::new());

        journal.mark_deleted("fingerprint1")?;
        let expired_resources2 = journal.record_usage("fingerprint2")?;
        assert_eq!(expired_resources2, Vec::<String>::new());

        Ok(())
    }
}
