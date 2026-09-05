use std::sync::atomic::{AtomicBool, Ordering};

/// Keep network synchronization and local mutations mutually exclusive without
/// holding a database/mutex guard across an await point.
#[derive(Default)]
pub struct OperationGate(AtomicBool);

pub struct OperationGuard<'a>(&'a AtomicBool);

impl OperationGate {
    pub fn enter(&self) -> Result<OperationGuard<'_>, String> {
        self.0.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "正在同步或保存数据，请完成后再试".to_string())?;
        Ok(OperationGuard(&self.0))
    }
}

impl Drop for OperationGuard<'_> {
    fn drop(&mut self) { self.0.store(false, Ordering::Release); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prevents_overlapping_operations_and_releases_on_drop() {
        let gate = OperationGate::default();
        let guard = gate.enter().unwrap();
        assert!(gate.enter().is_err());
        drop(guard);
        assert!(gate.enter().is_ok());
    }
}
