use once_cell::sync::Lazy;
use std::sync::atomic::AtomicU32;
use tokio::runtime::Runtime;

pub static TOKIO: Lazy<Runtime> = Lazy::new(|| new_runtime(None, None));

static NUM_IO_THREADS: AtomicU32 = AtomicU32::new(0);
static NUM_IDLE_IO_THREADS: AtomicU32 = AtomicU32::new(0);

/// Instantiate a new runtime.
pub fn new_runtime(worker_threads: Option<usize>, max_blocking_threads: Option<usize>) -> Runtime {
    // we want to use multiple threads
    let mut builder = tokio::runtime::Builder::new_multi_thread();

    builder
        // we use both IO and Time tokio utilities
        .enable_all()
        // give our threads a descriptive name (they'll be numbered too)
        .thread_name("holochain-tokio-thread");

    if let Some(worker_threads) = worker_threads {
        builder.worker_threads(worker_threads);
    };

    if let Some(max_blocking_threads) = max_blocking_threads {
        builder.max_blocking_threads(max_blocking_threads);
    };

    builder.on_thread_start(|| {
        let v = NUM_IO_THREADS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        println!("Launched new thread, now have {}", v + 1);
    });

    builder.on_thread_stop(|| {
        let v = NUM_IO_THREADS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        println!("Stopped a thread, now have {}", v - 1);

        // let v = NUM_IDLE_IO_THREADS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        // println!("Stopped a thread, now have {} idle", v - 1);
    });

    // builder.on_thread_park(|| {
    //     let v = NUM_IDLE_IO_THREADS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    //     println!("Parked a thread, now have {} idle", v + 1);
    // });
    //
    // builder.on_thread_unpark(|| {
    //     let v = NUM_IDLE_IO_THREADS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    //     println!("Un-parked a thread, now have {} idle", v);
    // });

    builder
        // build the runtime
        .build()
        // panic if we cannot (we cannot run without it)
        .expect("can build tokio runtime")
}

fn block_on_given<F>(f: F, runtime: &Runtime) -> F::Output
where
    F: futures::future::Future,
{
    let _g = runtime.enter();
    tokio::task::block_in_place(|| runtime.block_on(async { f.await }))
}

/// Run a blocking thread on `TOKIO` with a timeout.
pub fn block_on<F>(
    f: F,
    timeout: std::time::Duration,
) -> Result<F::Output, tokio::time::error::Elapsed>
where
    F: futures::future::Future,
{
    block_on_given(async { tokio::time::timeout(timeout, f).await }, &TOKIO)
}

/// Run a blocking thread on `TOKIO`.
pub fn block_forever_on<F>(f: F) -> F::Output
where
    F: futures::future::Future,
{
    block_on_given(f, &TOKIO)
}

/// Run a a task on the `TOKIO` static runtime.
pub fn run_on<F>(f: F) -> F::Output
where
    F: futures::future::Future,
{
    TOKIO.block_on(f)
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn block_forever_on_works() {
        block_forever_on(async { println!("stdio can block") });
        assert_eq!(1, super::block_forever_on(async { 1 }));

        let r = "1";
        let test1 = super::block_forever_on(async { r.to_string() });
        assert_eq!("1", &test1);

        // - wasm style use case -
        // we are in a non-tokio context
        let test2 = std::thread::spawn(|| {
            let r = "2";
            super::block_forever_on(async { r.to_string() })
        })
        .join()
        .unwrap();
        assert_eq!("2", &test2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn block_on_allows_spawning() {
        let r = "works";
        let test = block_forever_on(tokio::task::spawn(async move { r.to_string() })).unwrap();
        assert_eq!("works", &test);
    }

    // test calling without an existing reactor
    #[test]
    fn block_on_works() {
        assert_eq!(
            Ok(1),
            block_on(async { 1 }, std::time::Duration::from_millis(0))
        );
    }
}
