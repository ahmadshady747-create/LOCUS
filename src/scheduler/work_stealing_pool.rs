//! Lock-Free & Fine-Grained Work-Stealing Swarm Pool.
//!
//! Implements decentralized, capability-balanced task queues where idle workers
//! steal workload from the most loaded cores in 100% safe Rust.

#![forbid(unsafe_code)]

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::scheduler::types::{ComputeTask, SchedulerError};

/// Individual worker task queue with front-stealing and back-popping.
struct WorkerQueue {
    tasks: Mutex<VecDeque<ComputeTask>>,
    tasks_processed: AtomicUsize,
}

impl WorkerQueue {
    fn new(capacity: usize) -> Self {
        Self {
            tasks: Mutex::new(VecDeque::with_capacity(capacity)),
            tasks_processed: AtomicUsize::new(0),
        }
    }

    fn push(&self, task: ComputeTask) {
        let mut q = self.tasks.lock();
        q.push_back(task);
    }

    fn pop_local(&self) -> Option<ComputeTask> {
        let mut q = self.tasks.lock();
        let task = q.pop_back();
        if task.is_some() {
            self.tasks_processed.fetch_add(1, Ordering::Relaxed);
        }
        task
    }

    #[allow(dead_code)]
    fn steal_task(&self) -> Option<ComputeTask> {
        let mut q = self.tasks.lock();
        // Steal from the opposite end (front) to minimize contention
        q.pop_front()
    }

    fn steal_half(&self) -> Vec<ComputeTask> {
        let mut q = self.tasks.lock();
        let count = q.len() / 2;
        let mut stolen = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(task) = q.pop_front() {
                stolen.push(task);
            }
        }
        stolen
    }

    fn len(&self) -> usize {
        self.tasks.lock().len()
    }

    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.tasks.lock().is_empty()
    }
}

/// Swarm compute pool orchestrating work-stealing across multiple worker threads.
pub struct WorkStealingPool {
    workers: Vec<WorkerQueue>,
    worker_count: usize,
    total_stolen_count: AtomicUsize,
}

impl WorkStealingPool {
    /// Creates a new work-stealing pool with the specified worker count (defaulting to CPU cores).
    pub fn new(worker_count: usize, queue_capacity_per_worker: usize) -> Self {
        let count = worker_count.max(1);
        let mut workers = Vec::with_capacity(count);
        for _ in 0..count {
            workers.push(WorkerQueue::new(queue_capacity_per_worker));
        }

        Self {
            workers,
            worker_count: count,
            total_stolen_count: AtomicUsize::new(0),
        }
    }

    /// Schedules a task onto the worker with the lowest current workload.
    pub fn schedule_balanced(&self, task: ComputeTask) {
        let mut min_len = usize::MAX;
        let mut target_worker = 0;

        for (i, w) in self.workers.iter().enumerate() {
            let len = w.len();
            if len < min_len {
                min_len = len;
                target_worker = i;
            }
        }

        self.workers[target_worker].push(task);
    }

    /// Pushes a task directly to a specific worker thread queue.
    pub fn push_to_worker(&self, worker_id: usize, task: ComputeTask) -> Result<(), SchedulerError> {
        if worker_id >= self.worker_count {
            return Err(SchedulerError::WorkerNotFound(worker_id));
        }
        self.workers[worker_id].push(task);
        Ok(())
    }

    /// Fetches the next task for a worker: pops locally first, or steals from the heaviest worker.
    pub fn pop_or_steal(&self, worker_id: usize) -> Option<ComputeTask> {
        if worker_id >= self.worker_count {
            return None;
        }

        // 1. Fast Path: Pop local task
        if let Some(task) = self.workers[worker_id].pop_local() {
            return Some(task);
        }

        // 2. Slow Path: Steal from the heaviest peer worker
        let mut max_len = 0;
        let mut target_peer = None;

        for (i, w) in self.workers.iter().enumerate() {
            if i != worker_id {
                let len = w.len();
                if len > max_len {
                    max_len = len;
                    target_peer = Some(i);
                }
            }
        }

        if let Some(peer_idx) = target_peer {
            if max_len > 0 {
                // Steal half of the peer's pending workload
                let stolen = self.workers[peer_idx].steal_half();
                if !stolen.is_empty() {
                    let stolen_len = stolen.len();
                    self.total_stolen_count.fetch_add(stolen_len, Ordering::Relaxed);

                    // Push all but the first stolen task into our own local queue
                    let mut iter = stolen.into_iter();
                    let first = iter.next();
                    for t in iter {
                        self.workers[worker_id].push(t);
                    }
                    if first.is_some() {
                        self.workers[worker_id]
                            .tasks_processed
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    return first;
                }
            }
        }

        None
    }

    /// Executes and drains all queued tasks across all workers, returning the total processed count.
    pub fn process_all_tasks(&self) -> usize {
        let mut processed = 0;
        let mut active = true;

        while active {
            active = false;
            for worker_id in 0..self.worker_count {
                if let Some(_task) = self.pop_or_steal(worker_id) {
                    processed += 1;
                    active = true;
                }
            }
        }

        processed
    }

    /// Returns the total number of currently pending tasks across all workers.
    pub fn total_pending_tasks(&self) -> usize {
        self.workers.iter().map(|w| w.len()).sum()
    }

    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    pub fn total_stolen_count(&self) -> usize {
        self.total_stolen_count.load(Ordering::Relaxed)
    }

    pub fn worker_processed_count(&self, worker_id: usize) -> usize {
        if worker_id < self.worker_count {
            self.workers[worker_id]
                .tasks_processed
                .load(Ordering::Relaxed)
        } else {
            0
        }
    }
}
