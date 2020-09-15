use crossbeam_channel::{unbounded, Sender, Receiver};

pub struct Request<T> {
    tx: Sender<T>,
    rx: Receiver<T>,
}


impl<T> Request<T> {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            tx, rx
        }
    }

    pub fn get_handle(&self) -> Sender<T> {
        return self.tx.clone();
    }

    pub fn get_next(&self) -> Option<T> {
        match self.rx.try_recv() {
            Ok(data) => return Some(data),
            Err(_) => return None
        }
    }
}