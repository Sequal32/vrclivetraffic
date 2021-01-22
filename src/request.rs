use std::{thread, sync::Arc};

use crossbeam_channel::{unbounded, Sender, Receiver};
use crossbeam_deque::{Worker, Steal};

pub struct Request<T, J> {
    tx: Sender<T>,
    rx: Receiver<T>,
    worker: Worker<J>,
    num_threads: u32
}


impl<T, J> Request<T, J> where T: Send + 'static, J: Send + 'static {
    pub fn new(num_threads: u32) -> Self {
        let (tx, rx) = unbounded();
        Self {
            rx, tx, 
            worker: Worker::new_fifo(),
            num_threads
        }
    }

    pub fn run<F>(&self, worker: F) where
    F: Fn(J) -> T + Send + Sync + 'static {
        let worker = Arc::new(worker);

        // Spawn worker threads to read from queue
        (0..self.num_threads).for_each(|_| {
                let s = self.worker.stealer();
                let result_transmitter = self.tx.clone();
                let worker = worker.clone();
                // Process tasks
                thread::spawn(move || {
                    loop {
                        match s.steal() {
                            Steal::Success(job) => {
                                result_transmitter.send(worker(job)).ok();
                            },
                            _ => ()
                        }
                        thread::sleep(std::time::Duration::from_millis(500));
                    }
                });
            }
        );
    }

    pub fn get_next(&self) -> Option<T> {
        match self.rx.try_recv() {
            Ok(data) => return Some(data),
            Err(_) => return None
        }
    }

    pub fn give_job(&self, job: J) {
        self.worker.push(job);
    }
}