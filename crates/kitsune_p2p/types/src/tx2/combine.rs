use crate::tx2::tx2_adapter::*;
use crate::tx2::tx2_utils::*;
use crate::*;
use futures::future::{BoxFuture, FutureExt};
use futures::stream::{BoxStream, StreamExt};

thread_local!(static COMBINE_IDX: std::cell::RefCell<usize> = std::cell::RefCell::new(0));

/// invoke a combine api specifying the index of the sub-adapter to use
pub fn with_combine_idx<R, F>(idx: usize, f: F) -> R
where
    F: FnOnce() -> R,
{
    COMBINE_IDX.with(|s| {
        *s.borrow_mut() = idx;
        let r = f();
        *s.borrow_mut() = 0;
        r
    })
}

/// construct a comibned adapter factory with multiple sub factories
/// note, for instances created with this method, you may use the
/// `with_combine_idx` function to specify which adapter to control,
/// you may want to have the first one be the most important, given
/// the default adapter is the one with index zero.
pub fn tx2_combine_adapter<I: IntoIterator<Item = AdapterFactory>>(
    i: I,
) -> AdapterFactory {
    CombineBackendAdapt::new(i)
}

// -- private -- //

fn get_combine_idx() -> usize {
    COMBINE_IDX.with(|s| *s.borrow())
}

struct CombineConRecvAdapt(BoxStream<'static, ConFut>);

impl CombineConRecvAdapt {
    pub fn new(subs: Vec<Box<dyn ConRecvAdapt>>) -> Self {
        Self(futures::stream::select_all(subs).boxed())
    }
}

impl futures::stream::Stream for CombineConRecvAdapt {
    type Item = ConFut;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let inner = &mut self.0;
        tokio::pin!(inner);
        futures::stream::Stream::poll_next(inner, cx)
    }
}

impl ConRecvAdapt for CombineConRecvAdapt {}

struct CombineEndpointAdapt(Vec<Arc<dyn EndpointAdapt>>);

impl EndpointAdapt for CombineEndpointAdapt {
    fn debug(&self) -> serde_json::Value {
        self.0[get_combine_idx()].debug()
    }

    fn uniq(&self) -> Uniq {
        self.0[get_combine_idx()].uniq()
    }

    fn local_addr(&self) -> KitsuneResult<TxUrl> {
        self.0[get_combine_idx()].local_addr()
    }

    fn local_cert(&self) -> Tx2Cert {
        self.0[get_combine_idx()].local_cert()
    }

    fn connect(&self, url: TxUrl, timeout: KitsuneTimeout) -> ConFut {
        self.0[get_combine_idx()].connect(url, timeout)
    }

    fn is_closed(&self) -> bool {
        self.0[get_combine_idx()].is_closed()
    }

    fn close(&self, code: u32, reason: &str) -> BoxFuture<'static, ()> {
        self.0[get_combine_idx()].close(code, reason)
    }
}

/// Merge multiple backend adapts, used for mock testing
struct CombineBackendAdapt(Vec<AdapterFactory>);

impl CombineBackendAdapt {
    /// Construct a new memory-based test endpoint adapter for kitsune tx2.
    pub fn new<I: IntoIterator<Item = AdapterFactory>>(
        i: I,
    ) -> AdapterFactory {
        let out: AdapterFactory = Arc::new(Self(i.into_iter().collect()));
        out
    }
}

impl BindAdapt for CombineBackendAdapt {
    fn bind(&self, url: TxUrl, timeout: KitsuneTimeout) -> EndpointFut {
        let futs = self
            .0
            .iter()
            .map(|f| f.bind(url.clone(), timeout))
            .collect::<Vec<_>>();
        async move {
            let (ep, recv): (Vec<Arc<dyn EndpointAdapt>>, Vec<Box<dyn ConRecvAdapt>>) =
                futures::future::try_join_all(futs)
                    .await?
                    .into_iter()
                    .unzip();
            let ep: Arc<dyn EndpointAdapt> = Arc::new(CombineEndpointAdapt(ep));
            let recv: Box<dyn ConRecvAdapt> = Box::new(CombineConRecvAdapt::new(recv));
            Ok((ep, recv))
        }
        .boxed()
    }

    fn local_cert(&self) -> Tx2Cert {
        self.0[get_combine_idx()].local_cert()
    }
}
