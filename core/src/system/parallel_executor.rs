use std::{
    collections::HashMap,
    num::NonZeroUsize,
    panic::UnwindSafe,
    sync::mpsc::{channel, Receiver, Sender},
    thread::{Scope, ScopedJoinHandle},
};

use blaze_common::{error::Result, parallelism::Parallelism};

use super::thread::{join, thread};

const DEFAULT_THREADS_CAPACITY: usize = 16;

/// Used to execute functions in parallel in a thread pool and drain results asynchronously.
pub struct ParallelRunner<'scope, 'env: 'scope, T>
where
    T: Send + Sync,
{
    scope: &'scope Scope<'scope, 'env>,
    threads: HashMap<usize, ScopedJoinHandle<'scope, Result<T>>>,
    max: Option<NonZeroUsize>,
    thread_id_sequence: usize,
    termination_send: Sender<usize>,
    termination_recv: Receiver<usize>,
}

impl<'scope, 'env: 'scope, T> ParallelRunner<'scope, 'env, T>
where
    T: Send + Sync + UnwindSafe + 'scope,
{
    /// Create a new [`ParallelExecutor`] with the provided thread scope and parallelism level.
    pub fn new(scope: &'scope Scope<'scope, 'env>, level: Parallelism) -> Result<Self> {
        let max = match level {
            Parallelism::Infinite => None,
            Parallelism::All => Some(std::thread::available_parallelism()?),
            Parallelism::Count(i) => Some(i),
            Parallelism::None => Some(NonZeroUsize::MIN),
        };
        let (termination_send, termination_recv) = channel::<usize>();
        Ok(Self {
            scope,
            threads: HashMap::with_capacity(
                max.map(NonZeroUsize::get)
                    .unwrap_or(DEFAULT_THREADS_CAPACITY),
            ),
            max,
            thread_id_sequence: usize::MIN,
            termination_recv,
            termination_send,
        })
    }

    /// Push jobs to the executions threads pool for each available slots in the thread pool.
    /// Available slots are calculated according to the configured parallelism level.
    /// The execution supplier closure will be called for each slot available in the thread pool, or until [`None`] is returned by the closure, meaning that there are no more jobs pending.
    pub fn push_available<E, S>(&mut self, mut execution_supplier: S)
    where
        E: FnOnce() -> T + Send + UnwindSafe + 'scope,
        S: FnMut() -> Option<E>,
    {
        let available_slots = self
            .max
            .map(|m| m.get() - self.threads.len())
            .unwrap_or(usize::MAX);
        for _ in 0..available_slots {
            match execution_supplier() {
                Some(execution_routine) => {
                    let thread_id = self.thread_id_sequence;
                    let termination_send_clone = self.termination_send.clone();
                    self.threads.insert(
                        self.thread_id_sequence,
                        thread!(self.scope, move || {
                            let result = std::panic::catch_unwind(execution_routine);
                            termination_send_clone.send(thread_id)?;
                            Ok(match result {
                                Ok(v) => v,
                                Err(panic_value) => std::panic::resume_unwind(panic_value),
                            })
                        }),
                    );
                    self.thread_id_sequence += 1;
                }
                None => break,
            }
        }
    }

    /// Drain results from the current executions pool.
    /// Calling this function will block until at least **one** thread has been joined.
    pub fn drain(&mut self) -> Result<Vec<T>> {
        let mut drained = Vec::with_capacity(self.threads.len());

        let mut join = |thread_id| join!(self.threads.remove(&thread_id).unwrap());

        drained.push(join(self.termination_recv.recv()?)?);
        drained.extend(
            self.termination_recv
                .try_iter()
                .map(join)
                .collect::<Result<Vec<_>>>()?,
        );

        Ok(drained)
    }

    pub fn is_running(&self) -> bool {
        !self.threads.is_empty()
    }
}
