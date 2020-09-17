use std::thread;

use crossbeam_channel::{unbounded, Sender, Receiver};
use crossbeam_deque::{Worker, Steal};

pub struct Request<T, J> {
    tx: Sender<T>,
    rx: Receiver<T>,
    worker: Worker<J>
}


impl<T, J> Request<T, J> where T: Send + 'static, J: Send + 'static {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            rx, tx, 
            worker: Worker::new_fifo()
        }
    }

    pub fn run<F>(&self, worker: F) where
    F: Fn(J) -> T + Send + 'static {
        let result_transmitter = self.tx.clone();
        let s = self.worker.stealer();

        thread::spawn(move || {
            loop {
                match s.steal() {
                    Steal::Success(job) => {
                        result_transmitter.send(worker(job)).ok();
                    },
                    _ => ()
                }
                thread::sleep(std::time::Duration::from_millis(10));
            }
        });
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