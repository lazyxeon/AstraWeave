//! Deterministic query jobs.
//!
//! Workers are started with the world, not during a tick. Each job writes one
//! slot. The main thread merges slots in index order. The walk itself does not
//! take a lock. GPU is not used: an integer GPU solve would need its own
//! cross-OS replay proof before it could touch simulation state.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};

use crate::octree::{solid_in_image, CHUNK_EXTENT};

const WORKERS: usize = 2;
const MAX_NODES: usize = 2048;
pub const MAX_QUERIES: usize = 64;

struct Image {
    children: [[u16; 8]; MAX_NODES],
    density: [i8; MAX_NODES],
    leaf: [u8; MAX_NODES],
    len: u16,
    root: u16,
    origin: [i32; 3],
    points: [[i32; 3]; MAX_QUERIES],
    count: usize,
}

struct Phase {
    running: bool,
    cursor: usize,
    done: usize,
    count: usize,
    shutdown: bool,
}

struct Board {
    image: RwLock<Image>,
    phase: Mutex<Phase>,
    cv: Condvar,
    answers: [AtomicU8; MAX_QUERIES],
    stop: AtomicBool,
}

pub struct QueryPool {
    board: Arc<Board>,
    workers: Vec<JoinHandle<()>>,
}

impl QueryPool {
    pub fn new() -> Self {
        let board = Arc::new(Board {
            image: RwLock::new(Image {
                children: [[0u16; 8]; MAX_NODES],
                density: [0; MAX_NODES],
                leaf: [0; MAX_NODES],
                len: 0,
                root: 0,
                origin: [0, 0, 0],
                points: [[0; 3]; MAX_QUERIES],
                count: 0,
            }),
            phase: Mutex::new(Phase {
                running: false,
                cursor: 0,
                done: 0,
                count: 0,
                shutdown: false,
            }),
            cv: Condvar::new(),
            answers: std::array::from_fn(|_| AtomicU8::new(0)),
            stop: AtomicBool::new(false),
        });
        let mut workers = Vec::with_capacity(WORKERS);
        for _ in 0..WORKERS {
            let board = Arc::clone(&board);
            workers.push(thread::spawn(move || worker(board)));
        }
        Self { board, workers }
    }

    /// Publish a DAG image and query points. Returns solid flags in point order.
    pub fn query_solid(
        &self,
        children: &[[u16; 8]],
        density: &[i8],
        leaf: &[u8],
        len: u16,
        root: u16,
        origin: [i32; 3],
        points: &[[i32; 3]],
        out: &mut [u8],
    ) {
        let count = points.len().min(out.len()).min(MAX_QUERIES);
        {
            let mut image = self
                .board
                .image
                .write()
                .unwrap_or_else(|err| err.into_inner());
            let n = (len as usize).min(MAX_NODES).min(children.len());
            image.children[..n].copy_from_slice(&children[..n]);
            image.density[..n].copy_from_slice(&density[..n]);
            image.leaf[..n].copy_from_slice(&leaf[..n]);
            image.len = n as u16;
            image.root = root;
            image.origin = origin;
            for (index, point) in points.iter().take(count).enumerate() {
                image.points[index] = *point;
            }
            image.count = count;
        }
        for answer in self.board.answers.iter().take(count) {
            answer.store(0, Ordering::Relaxed);
        }
        {
            let mut phase = self
                .board
                .phase
                .lock()
                .unwrap_or_else(|err| err.into_inner());
            phase.cursor = 0;
            phase.done = 0;
            phase.count = count;
            phase.running = count > 0;
            self.board.cv.notify_all();
            while phase.running {
                phase = self
                    .board
                    .cv
                    .wait(phase)
                    .unwrap_or_else(|err| err.into_inner());
            }
        }
        for index in 0..count {
            out[index] = self.board.answers[index].load(Ordering::Relaxed);
        }
    }
}

impl Default for QueryPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for QueryPool {
    fn drop(&mut self) {
        self.board.stop.store(true, Ordering::Relaxed);
        {
            let mut phase = self
                .board
                .phase
                .lock()
                .unwrap_or_else(|err| err.into_inner());
            phase.shutdown = true;
            phase.running = false;
            self.board.cv.notify_all();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn worker(board: Arc<Board>) {
    loop {
        let job = {
            let mut phase = board.phase.lock().unwrap_or_else(|err| err.into_inner());
            loop {
                if phase.shutdown || board.stop.load(Ordering::Relaxed) {
                    return;
                }
                if phase.running && phase.cursor < phase.count {
                    let job = phase.cursor;
                    phase.cursor += 1;
                    break job;
                }
                phase = board.cv.wait(phase).unwrap_or_else(|err| err.into_inner());
            }
        };
        let answer = {
            let image = board.image.read().unwrap_or_else(|err| err.into_inner());
            let point = image.points[job];
            match solid_in_image(
                &image.children[..image.len as usize],
                &image.density[..image.len as usize],
                &image.leaf[..image.len as usize],
                image.root,
                image.origin,
                point[0],
                point[1],
                point[2],
            ) {
                Some(true) => 2,
                Some(false) => 1,
                None => 0,
            }
        };
        board.answers[job].store(answer, Ordering::Relaxed);
        let mut phase = board.phase.lock().unwrap_or_else(|err| err.into_inner());
        phase.done += 1;
        if phase.done >= phase.count {
            phase.running = false;
            board.cv.notify_all();
        }
    }
}

pub fn chunk_origin(chunk_x: i32, chunk_z: i32) -> [i32; 3] {
    [chunk_x * CHUNK_EXTENT, 0, chunk_z * CHUNK_EXTENT]
}
