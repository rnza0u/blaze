use std::{
    fmt::{Debug, Display},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{Scope, ScopedJoinHandle},
};

use super::random::random_string;
use super::thread::{join, thread};
use anyhow::Context;
use blaze_common::error::{Error, Result};
use interprocess::local_socket::{
    traits::Listener, GenericFilePath, Listener as LocalSocketListener, ListenerNonblockingMode,
    ListenerOptions, Stream, ToFsName,
};

pub struct IpcServer<'scope> {
    path: PathBuf,
    main_thread_join_handle: ScopedJoinHandle<'scope, Result<()>>,
    stopped: Arc<AtomicBool>,
}

impl<'scope> IpcServer<'scope> {
    pub fn get_path(&self) -> &Path {
        &self.path
    }

    #[cfg(unix)]
    fn create_path() -> PathBuf {
        let mut tmp = std::env::temp_dir();
        tmp.extend(&PathBuf::from(&format!("blaze_{}.sock", random_string(12))));
        tmp
    }

    #[cfg(windows)]
    fn create_path() -> PathBuf {
        let mut tmp = std::env::temp_dir();
        tmp.extend(&PathBuf::from(&format!(
            "\\\\.\\pipe\\LOCAL\\blaze_{}",
            random_string(12)
        )));
        tmp
    }

    pub fn create<'env, C, E>(
        scope: &'scope Scope<'scope, 'env>,
        client_handler: C,
        error_handler: E,
    ) -> Result<Self>
    where
        C: FnOnce(IpcClientConnection) -> Result<()> + Send + Clone + 'scope,
        E: Fn(Error) + Send + Clone + 'scope,
    {
        let stopped_0 = Arc::new(AtomicBool::new(false));
        let stopped_1 = stopped_0.clone();

        let path = Self::create_path();

        let mut listener =
            IpcListener::create(&path).context("error while creating IPC listener")?;

        let main_thread = thread!(scope, move || {
            let mut client_threads: Vec<ScopedJoinHandle<Result<()>>> = vec![];

            let join_client_thread = |thread: ScopedJoinHandle<Result<()>>| {
                if let Err(err) = join!(thread) {
                    error_handler(err);
                }
            };

            while !stopped_1.load(Ordering::SeqCst) {
                if let Some(client) = listener
                    .try_accept_client()
                    .context("IPC listener could not accept client")?
                {
                    let cloned_handler = client_handler.clone();
                    client_threads.push(thread!(scope, move || cloned_handler(client)));
                };

                for i in (0..client_threads.len()).rev() {
                    if client_threads[i].is_finished() {
                        let removed = client_threads.remove(i);
                        join_client_thread(removed);
                    }
                }
            }

            for thread in client_threads {
                join_client_thread(thread);
            }

            Ok(())
        });

        Ok(Self {
            path,
            main_thread_join_handle: main_thread,
            stopped: stopped_0,
        })
    }

    pub fn close(self) -> Result<()> {
        self.stopped.store(true, Ordering::SeqCst);
        join!(self.main_thread_join_handle)
            .context("error while closing IPC server main loop thread")?;
        Ok(())
    }
}

impl Display for IpcServer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path.fmt(f)
    }
}

struct IpcListener(LocalSocketListener);

impl IpcListener {
    fn create(path: &Path) -> Result<Self> {
        let listener = ListenerOptions::new()
            .nonblocking(ListenerNonblockingMode::Accept)
            .name(ToFsName::to_fs_name::<GenericFilePath>(path)?)
            .create_sync()?;
        Ok(Self(listener))
    }

    fn try_accept_client(&mut self) -> Result<Option<IpcClientConnection>> {
        match self.0.accept() {
            Ok(stream) => Ok(Some(IpcClientConnection { stream })),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    Ok(None)
                } else {
                    Err(err).context("could not accept IPC connections from socket")
                }
            }
        }
    }
}

pub struct IpcClientConnection {
    stream: Stream,
}

impl std::io::Read for IpcClientConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}
