use std::{
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
    thread,
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use crossbeam_deque::{Steal, Worker};

pub struct Request<T, J> {
    tx: Sender<T>,
    rx: Receiver<T>,
    worker: Worker<J>,
    num_threads: u32,
    running: Arc<AtomicBool>,
}

impl<T, J> Request<T, J>
where
    T: Send + 'static,
    J: Send + 'static,
{
    pub fn new(num_threads: u32) -> Self {
        let (tx, rx) = unbounded();
        Self {
            rx,
            tx,
            worker: Worker::new_fifo(),
            num_threads,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn run<F>(&self, worker: F)
    where
        F: Fn(J) -> T + Send + Sync + 'static,
    {
        let worker = Arc::new(worker);

        self.running.store(false, SeqCst);

        // Spawn worker threads to read from queue
        (0..self.num_threads).for_each(|_| {
            let s = self.worker.stealer();
            let result_transmitter = self.tx.clone();
            let worker = worker.clone();

            let running_clone = self.running.clone();

            // Process tasks
            thread::spawn(move || loop {
                match s.steal() {
                    Steal::Success(job) => {
                        result_transmitter.send(worker(job)).ok();
                    }
                    _ => (),
                }

                if running_clone.load(SeqCst) {
                    break;
                }

                thread::sleep(std::time::Duration::from_millis(500));
            });
        });
    }

    pub fn stop(&self) {
        self.running.store(true, SeqCst);
    }

    pub fn get_next(&self) -> Option<T> {
        match self.rx.try_recv() {
            Ok(data) => return Some(data),
            Err(_) => return None,
        }
    }

    pub fn give_job(&self, job: J) {
        self.worker.push(job);
    }
}
