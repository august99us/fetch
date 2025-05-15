use std::time::{Duration, Instant};

use list::SyncedLinkedList;
use tokio::sync::watch::{Receiver, Sender};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Task<D: Clone> {
    id: String,
    data: D,
    checkout: bool,
    checkout_timeout: Option<Instant>,
    attempts: u32,
}

impl<D: Clone> Task<D> {
    pub fn new(data: D) -> Self {
        Task {
            id: Uuid::new_v4().to_string(),
            data,
            checkout: false,
            checkout_timeout: None,
            attempts: 0,
        }
    }

    pub fn checked_out(&self) -> bool {
        self.checkout
    }

    pub fn with_checkedout_duration(&mut self, duration: &Duration) -> Result<Task<D>, String> {
        if self.checkout {
            return Err("Task already checked out".to_string());
        }
        let mut clone = self.clone();
        clone.checkout = true;
        clone.checkout_timeout = Some(Instant::now() + *duration);
        Ok(clone)
    }
}

pub fn default_ingestor_worker_queue<L: SyncedLinkedList<Task<D>> + Sync + Send, D: Clone>() -> (Ingestor<L, D>, Distributor<L, D>) {
    let list = BasicList::new();
    let (tx, rx) = tokio::sync::watch::channel(0);
    let ingestor = Ingestor { list: list.clone(), ingest_num: 0, broadcaster: tx };
    let worker = Distributor {
        list: list.clone(),
        checkout_duration: Duration::from_secs(60),
        watcher: rx,
    };
    (ingestor, worker)
}

pub trait TaskQueue<D: Clone> {
    fn queue_task(&mut self, task: Task<D>) -> Result<(), anyhow::Error>;
    fn request_task(&self) -> Result<Task<D>, anyhow::Error>;
    fn cancel_task(&self, id: String) -> Result<(), anyhow::Error>;
    fn complete_task(&self, id: String) -> Result<(), anyhow::Error>;
}

pub struct Ingestor<Q: TaskQueue<D> + Send, D: Clone> {
    list: Q,
    ingest_num: i32,
    broadcaster: Sender<i32>,
}

impl<Q: TaskQueue<D> + Send, D: Clone> Ingestor<L, D> {
    pub fn queue_task(&mut self, task_data: D) -> Result<(), anyhow::Error> {
        // Implementation for queueing a task
        self.list.queue_task(Task::new(task_data))?;
        self.ingest_num += 1;
        self.broadcaster.send(self.ingest_num).map_err(|e| anyhow::anyhow!("Failed to send task: {:?}", e))?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Distributor<L: SyncedLinkedList<Task<D>> + Sync + Send, D: Clone> {
    list: L,
    checkout_duration: Duration,
    watcher: Receiver<i32>,
}

impl<L: SyncedLinkedList<Task<D>> + Sync + Send, D: Clone> Distributor<L, D> {
    pub fn request_task(&self) -> Result<Task<D>, anyhow::Error> {
        // Implementation for requesting a task
        loop {
            for item in self.list.iter_mut() {
                let task = item?.get();
                if !task.checkout {
                    let checkout_task = task.with_checkedout_duration(&self.checkout_duration)?;
                    item.set(checkout_task)?;
                    return Ok(checkout_task);
                }
            }
            self.watcher.changed().await?;
        }
    }
    pub fn cancel_task(&self, id: String) -> Result<(), anyhow::Error> {
        // Implementation for canceling a task
        todo!()
    }
    pub fn complete_task(&self, id: String) -> Result<(), anyhow::Error> {
        // Implementation for completing a task
        todo!()
    }
}

pub mod list;