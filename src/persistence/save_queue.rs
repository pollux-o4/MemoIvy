use super::{note::Note, store};
use std::{path::PathBuf, sync::mpsc, thread};

#[derive(Clone, Copy)]
enum Job {
    Snapshot(u64),
    Transaction,
}

pub struct SaveQueue {
    sender: mpsc::Sender<(Job, Vec<Note>)>,
    receiver: mpsc::Receiver<(Job, Result<(), String>)>,
    revision: u64,
    durable: u64,
    in_flight: bool,
    failed: bool,
    dispatch_failure: Option<Job>,
    active_job: Option<Job>,
}
impl SaveQueue {
    pub fn new(path: PathBuf) -> Self {
        let (sender, jobs) = mpsc::channel::<(Job, Vec<Note>)>();
        let (results, receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok((revision, notes)) = jobs.recv() {
                let result = store::save(&path, &notes).map_err(|e| e.to_string());
                if results.send((revision, result)).is_err() {
                    break;
                }
            }
        });
        Self {
            sender,
            receiver,
            revision: 0,
            durable: 0,
            in_flight: false,
            failed: false,
            dispatch_failure: None,
            active_job: None,
        }
    }
    pub fn changed(&mut self) {
        self.revision += 1;
        self.failed = false;
    }
    pub fn dispatch(&mut self, notes: &[Note]) {
        if !self.in_flight && self.revision != self.durable {
            self.in_flight = true;
            self.active_job = Some(Job::Snapshot(self.revision));
            if self
                .sender
                .send((Job::Snapshot(self.revision), notes.to_vec()))
                .is_err()
            {
                self.in_flight = false;
                self.failed = true;
                self.dispatch_failure = Some(Job::Snapshot(self.revision));
            }
        }
    }
    pub fn transaction(&mut self, notes: &[Note]) {
        self.in_flight = true;
        self.active_job = Some(Job::Transaction);
        if self
            .sender
            .send((Job::Transaction, notes.to_vec()))
            .is_err()
        {
            self.in_flight = false;
            self.failed = true;
            self.dispatch_failure = Some(Job::Transaction);
        }
    }
    pub fn poll(&mut self) -> Option<(bool, Result<(), String>)> {
        let (job, result) = if let Some(job) = self.dispatch_failure.take() {
            (job, Err("background writer unavailable".into()))
        } else {
            match self.receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    let job = self.active_job.take()?;
                    (job, Err("background writer unavailable".into()))
                }
            }
        };
        self.active_job = None;
        self.in_flight = false;
        let transaction = matches!(job, Job::Transaction);
        match result {
            Ok(()) => {
                if let Job::Snapshot(revision) = job {
                    self.durable = revision;
                }
                self.failed = false;
                Some((transaction, Ok(())))
            }
            Err(e) => {
                self.failed = true;
                Some((transaction, Err(e)))
            }
        }
    }
    pub fn in_flight(&self) -> bool {
        self.in_flight || self.dispatch_failure.is_some()
    }
    pub fn failed(&self) -> bool {
        self.failed
    }
    pub fn retry(&mut self) {
        self.failed = false;
    }
    pub fn record_failure(&mut self) {
        self.failed = true;
    }
    pub fn clean(&self) -> bool {
        self.revision == self.durable && !self.in_flight && !self.failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    fn wait(queue: &mut SaveQueue) -> (bool, Result<(), String>) {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = queue.poll() {
                return result;
            }
            assert!(Instant::now() < end, "writer timed out");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn save_001_newer_revision_stays_dirty_until_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let mut queue = SaveQueue::new(path.clone());
        let mut note = Note::new();
        queue.changed();
        queue.dispatch(&[note.clone()]);
        note.text = "마지막 입력".into();
        queue.changed();
        assert!(wait(&mut queue).1.is_ok());
        assert!(!queue.clean());
        queue.dispatch(&[note.clone()]);
        assert!(wait(&mut queue).1.is_ok());
        assert!(queue.clean());
        assert_eq!(Store::load(path).unwrap().notes[0].text, note.text);
    }
    #[test]
    fn save_002_failure_remains_dirty_and_can_retry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        std::fs::create_dir(&path).unwrap();
        let mut queue = SaveQueue::new(path.clone());
        let notes = [Note::new()];
        queue.changed();
        queue.dispatch(&notes);
        assert!(wait(&mut queue).1.is_err());
        assert!(!queue.clean());
        std::fs::remove_dir(&path).unwrap();
        queue.retry();
        queue.dispatch(&notes);
        assert!(wait(&mut queue).1.is_ok());
        assert!(queue.clean());
    }
    #[test]
    fn note_002_transaction_then_latest_snapshot_preserves_other_edit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let mut queue = SaveQueue::new(path.clone());
        let deleted = Note::new();
        let mut other = Note::new();
        queue.transaction(&[other.clone()]);
        other.text = "동시 입력".into();
        queue.changed();
        assert!(wait(&mut queue).1.is_ok());
        assert!(!queue.clean());
        queue.dispatch(&[other.clone()]);
        assert!(wait(&mut queue).1.is_ok());
        let notes = Store::load(path).unwrap().notes;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].text, other.text);
        assert_ne!(notes[0].id, deleted.id);
    }
    #[test]
    fn perf_001_burst_coalesces_until_in_flight_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let mut queue = SaveQueue::new(path.clone());
        let mut note = Note::new();
        queue.changed();
        queue.dispatch(&[note.clone()]);
        for i in 0..100 {
            note.text = i.to_string();
            queue.changed();
            queue.dispatch(&[note.clone()]);
        }
        assert!(wait(&mut queue).1.is_ok());
        assert!(!queue.clean());
        queue.dispatch(&[note]);
        assert!(wait(&mut queue).1.is_ok());
        assert!(queue.clean());
        assert_eq!(Store::load(path).unwrap().notes[0].text, "99");
    }
    #[test]
    fn save_001_close_flush_waits_for_latest_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let mut queue = SaveQueue::new(path.clone());
        let mut note = Note::new();
        queue.changed();
        queue.dispatch(&[note.clone()]);
        note.text = "닫기 직전 입력".into();
        queue.changed();
        queue.retry();
        queue.dispatch(&[note.clone()]);
        assert!(wait(&mut queue).1.is_ok());
        assert!(!queue.clean());
        queue.dispatch(&[note.clone()]);
        assert!(wait(&mut queue).1.is_ok());
        assert!(queue.clean());
        assert_eq!(Store::load(path).unwrap().notes[0].text, note.text);
    }

    #[test]
    fn note_002_failed_transaction_preserves_original_and_retries_latest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        std::fs::create_dir(&path).unwrap();
        let mut queue = SaveQueue::new(path.clone());
        let original = Note::new();
        queue.transaction(&[]);
        let (transaction, result) = wait(&mut queue);
        assert!(transaction);
        assert!(result.is_err());
        assert!(queue.failed());
        assert!(path.is_dir());
        std::fs::remove_dir(&path).unwrap();
        queue.changed();
        queue.record_failure();
        assert!(!queue.clean());
        queue.retry();
        queue.dispatch(&[original.clone()]);
        let (transaction, result) = wait(&mut queue);
        assert!(!transaction);
        assert!(result.is_ok());
        assert!(queue.clean());
        assert_eq!(Store::load(path).unwrap().notes[0].id, original.id);
    }
    #[test]
    fn save_002_disconnected_writer_reports_failure_instead_of_waiting_forever() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = SaveQueue::new(dir.path().join("notes.json"));
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        queue.sender = sender;
        queue.changed();
        queue.dispatch(&[Note::new()]);
        assert!(queue.in_flight());
        assert!(queue.poll().unwrap().1.is_err());
        assert!(queue.failed());
        assert!(!queue.in_flight());
        assert!(queue.poll().is_none());
        queue.transaction(&[]);
        let (transaction, result) = queue.poll().unwrap();
        assert!(transaction);
        assert!(result.is_err());
    }
    #[test]
    fn save_002_disconnected_result_channel_reports_active_job() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = SaveQueue::new(dir.path().join("notes.json"));
        let (sender, receiver) = mpsc::channel();
        drop(sender);
        queue.receiver = receiver;
        queue.in_flight = true;
        queue.active_job = Some(Job::Transaction);
        let (transaction, result) = queue.poll().unwrap();
        assert!(transaction);
        assert!(result.is_err());
        assert!(queue.failed());
        assert!(!queue.in_flight());
        assert!(queue.poll().is_none());
    }
    use super::store::Store;
}
