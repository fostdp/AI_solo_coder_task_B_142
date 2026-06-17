use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use rayon::ThreadPoolBuilder;
use tokio::sync::oneshot;

use crate::errors::{AppError, Result};

lazy_static! {
    pub static ref GLOBAL_COMPUTE_POOL: ComputePool =
        ComputePool::new(ComputePoolConfig::default())
            .expect("Failed to build global compute pool");
}

#[derive(Debug, Clone)]
pub struct ComputePoolConfig {
    pub num_threads: usize,
    pub thread_name_prefix: String,
    pub stack_size: Option<usize>,
}

impl Default for ComputePoolConfig {
    fn default() -> Self {
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            num_threads: cpu_cores.min(8).max(2),
            thread_name_prefix: "sinan-micro-mag".to_string(),
            stack_size: Some(4 * 1024 * 1024),
        }
    }
}

pub struct ComputePool {
    config: ComputePoolConfig,
    pool: rayon::ThreadPool,
    submitted: Arc<AtomicU64>,
    completed: Arc<AtomicU64>,
    in_flight: Arc<Mutex<Vec<String>>>,
}

impl std::fmt::Debug for ComputePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputePool")
            .field("config", &self.config)
            .field("submitted", &self.submitted.load(Ordering::Relaxed))
            .field("completed", &self.completed.load(Ordering::Relaxed))
            .finish()
    }
}

impl ComputePool {
    pub fn new(config: ComputePoolConfig) -> Result<Self> {
        let prefix = config.thread_name_prefix.clone();
        let mut builder = ThreadPoolBuilder::new()
            .num_threads(config.num_threads)
            .thread_name(move |i| format!("{}-{}", prefix, i));
        if let Some(stack) = config.stack_size {
            builder = builder.stack_size(stack);
        }
        let pool = builder
            .build()
            .map_err(|e| AppError::InternalError(format!("Failed to build rayon pool: {}", e)))?;
        Ok(Self {
            config,
            pool,
            submitted: Arc::new(AtomicU64::new(0)),
            completed: Arc::new(AtomicU64::new(0)),
            in_flight: Arc::new(Mutex::new(Vec::with_capacity(32))),
        })
    }

    pub fn global() -> &'static Self {
        &GLOBAL_COMPUTE_POOL
    }

    pub fn config(&self) -> &ComputePoolConfig {
        &self.config
    }

    pub fn stats(&self) -> ComputePoolStats {
        ComputePoolStats {
            submitted: self.submitted.load(Ordering::Relaxed),
            completed: self.completed.load(Ordering::Relaxed),
            num_threads: self.config.num_threads as u64,
            in_flight_count: self.in_flight.lock().len() as u64,
        }
    }

    pub fn spawn_compute<F, R>(&self, name: &str, f: F) -> ComputeFuture<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel::<R>();
        let submitted = self.submitted.clone();
        let completed = self.completed.clone();
        let in_flight = self.in_flight.clone();
        let name_owned = name.to_string();
        let name_track = name.to_string();

        submitted.fetch_add(1, Ordering::Relaxed);
        {
            let mut guard = in_flight.lock();
            guard.push(name_owned);
        }

        self.pool.spawn(move || {
            let result = f();
            let _ = tx.send(result);
            completed.fetch_add(1, Ordering::Relaxed);
            let mut guard = in_flight.lock();
            if let Some(pos) = guard.iter().position(|x| x == &name_track) {
                guard.swap_remove(pos);
            }
        });

        ComputeFuture { rx: Box::pin(rx) }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ComputePoolStats {
    pub submitted: u64,
    pub completed: u64,
    pub num_threads: u64,
    pub in_flight_count: u64,
}

pub struct ComputeFuture<R> {
    rx: Pin<Box<oneshot::Receiver<R>>>,
}

impl<R> Future for ComputeFuture<R> {
    type Output = Result<R>;
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.rx.as_mut().poll(cx) {
            std::task::Poll::Ready(Ok(v)) => std::task::Poll::Ready(Ok(v)),
            std::task::Poll::Ready(Err(_)) => std::task::Poll::Ready(Err(
                AppError::InternalError("Compute task was cancelled".into()),
            )),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_pool_creation() {
        let cfg = ComputePoolConfig {
            num_threads: 2,
            thread_name_prefix: "test-pool".into(),
            stack_size: Some(2 * 1024 * 1024),
        };
        let p = ComputePool::new(cfg).unwrap();
        assert_eq!(p.config().num_threads, 2);
        let stats = p.stats();
        assert_eq!(stats.num_threads, 2);
        assert_eq!(stats.submitted, 0);
        assert_eq!(stats.completed, 0);
    }

    #[test]
    fn test_spawn_compute_sync() {
        let cfg = ComputePoolConfig {
            num_threads: 2,
            ..Default::default()
        };
        let p = ComputePool::new(cfg).unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        let c2 = counter.clone();

        std::thread::scope(|s| {
            s.spawn(|| {
                let _result = p.spawn_compute("add", move || {
                    c2.fetch_add(5, Ordering::SeqCst);
                    42
                });
            });
        });

        std::thread::sleep(std::time::Duration::from_millis(300));
        assert_eq!(counter.load(Ordering::SeqCst), 5);
        let stats = p.stats();
        assert_eq!(stats.submitted, 1);
        assert_eq!(stats.completed, 1);
    }
}
