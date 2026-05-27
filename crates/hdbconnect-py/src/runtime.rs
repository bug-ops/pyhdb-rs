//! Sync bridge: runs async futures on a dedicated tokio runtime.
//!
//! Used exclusively by the sync Python API (`PyConnection`, `PyCursor`).
//! Must never be called from inside a tokio worker thread.

use std::future::Future;
use std::sync::OnceLock;

static SYNC_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn sync_rt() -> &'static tokio::runtime::Runtime {
    SYNC_RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build sync tokio runtime")
    })
}

/// Run an async future to completion on the sync runtime.
///
/// Must only be called from Python main thread (non-tokio-worker).
pub(crate) fn block_on<F: Future>(f: F) -> F::Output {
    sync_rt().block_on(f)
}
