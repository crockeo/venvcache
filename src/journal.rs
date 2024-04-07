use std::path::Path;

// TODO: could this be done better with sqlite?
// it seems easier to keep an open connection and just do atomic operations
// and then let sqlite manage shared connections to the same file...

/// Provides an interface to a least frecency-used cache.
/// Allows one to journal the usage of resources (identified by fingerprints)
/// on disk, and calculate which resource is most optimal to delete.
pub struct Journal {}

impl Journal {
    pub fn new(path: &Path, maximum_resources: usize) -> anyhow::Result<Self> {
        // todo!()
        Ok(Self{})
    }

    /// Records that a resource was used.
    /// If the number of resources used exceeds the maximum allocated amount
    /// this function will also return a fingerprint whose resource should be deleted.
    /// That fingerprint is determined by the least-frecent (recent + frequent) resource.
    pub fn record_usage(&mut self, fingerprint: &str) -> anyhow::Result<Option<String>> {
        // todo!()
        Ok(None)
    }

    /// Marks a particular resource as deleted.
    pub fn mark_deleted(&mut self, fingerprint: &str) -> anyhow::Result<()> {
        // todo!()
        Ok(())
    }
}
