use crate::tx2::tx2_adapter::*;
use crate::tx2::tx2_utils::*;
use crate::*;
use futures::future::{BoxFuture, FutureExt};
use futures::stream::{BoxStream, StreamExt};
use parking_lot::Mutex;

/// callback hooks for mock functionality
pub struct MockConHooks {}

impl Default for MockConHooks {
    fn default() -> Self {
        Self {}
    }
}

/// callback hooks for mock functionality
pub struct MockInChanRecvHooks {}

impl Default for MockInChanRecvHooks {
    fn default() -> Self {
        Self {}
    }
}

type IsClosedSig = Box<dyn FnMut() -> bool + 'static + Send>;

/// callback hooks for mock functionality
pub struct MockEndpointHooks {
    is_closed: IsClosedSig,
}

impl MockEndpointHooks {
    /// default is_closed always returns false
    pub fn default_is_closed() -> bool {
        false
    }

    /// customize what happens is_closed is called on the endpoint
    pub fn set_is_closed<F: FnMut() -> bool + 'static + Send>(&mut self, f: F) {
        self.is_closed = Box::new(f);
    }
}

impl Default for MockEndpointHooks {
    fn default() -> Self {
        Self {
            is_closed: Box::new(MockEndpointHooks::default_is_closed),
        }
    }
}

/// callback hooks for mock functionality
pub struct MockConRecvHooks {
    stream: BoxStream<'static, KitsuneResult<(MockConHooks, MockInChanRecvHooks)>>,
}

impl MockConRecvHooks {
    /// customize what connection events are emitted by this mock
    pub fn set_incoming_connection_stream<
        S: futures::stream::Stream<Item = KitsuneResult<(MockConHooks, MockInChanRecvHooks)>>
            + 'static
            + Send,
    >(
        &mut self,
        s: S,
    ) {
        self.stream = s.boxed();
    }
}

impl Default for MockConRecvHooks {
    fn default() -> Self {
        Self {
            stream: futures::stream::pending().boxed(),
        }
    }
}

type BindSig = Box<
    dyn FnMut(TxUrl, KitsuneTimeout) -> KitsuneResult<(MockEndpointHooks, MockConRecvHooks)>
        + 'static
        + Send,
>;

type LocalCertSig = Box<dyn FnMut(Tx2Cert) -> Tx2Cert + 'static + Send>;

/// callback hooks for mock functionality
pub struct MockFactoryHooks {
    bind: BindSig,
    local_cert: LocalCertSig,
}

impl MockFactoryHooks {
    /// will build stub subtypes that do nothing
    pub fn default_bind(
        _url: TxUrl,
        _timeout: KitsuneTimeout,
    ) -> KitsuneResult<(MockEndpointHooks, MockConRecvHooks)> {
        Ok((Default::default(), Default::default()))
    }

    /// just returns the cert we were constructed with
    pub fn default_local_cert(tls: Tx2Cert) -> Tx2Cert {
        tls
    }

    /// customize what happens when a factory binds an endpoint
    pub fn set_bind<
        F: FnMut(TxUrl, KitsuneTimeout) -> KitsuneResult<(MockEndpointHooks, MockConRecvHooks)>
            + 'static
            + Send,
    >(
        &mut self,
        f: F,
    ) {
        self.bind = Box::new(f);
    }

    /// customize what happens local_cert is called on the factory
    pub fn set_local_cert<F: FnMut(Tx2Cert) -> Tx2Cert + 'static + Send>(&mut self, f: F) {
        self.local_cert = Box::new(f);
    }
}

impl Default for MockFactoryHooks {
    fn default() -> Self {
        Self {
            bind: Box::new(MockFactoryHooks::default_bind),
            local_cert: Box::new(MockFactoryHooks::default_local_cert),
        }
    }
}

/// construct a new mock adapter factory
pub fn tx2_mock_adapter(tls: Tx2Cert, f: MockFactoryHooks) -> AdapterFactory {
    MockBackendAdapt::new(tls, f)
}

// -- private -- //

struct MockConRecvAdapt(BoxStream<'static, ConFut>);

impl MockConRecvAdapt {
    pub fn new(
        sub: BoxStream<'static, KitsuneResult<(MockConHooks, MockInChanRecvHooks)>>,
    ) -> Self {
        Self(
            sub.map(|r| {
                async move {
                    let (_con, _recv) = r?;
                    todo!()
                }
                .boxed()
            })
            .boxed(),
        )
    }
}

impl futures::stream::Stream for MockConRecvAdapt {
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

impl ConRecvAdapt for MockConRecvAdapt {}

struct MockEndpointAdapt(Arc<Mutex<MockEndpointHooks>>);

impl EndpointAdapt for MockEndpointAdapt {
    fn debug(&self) -> serde_json::Value {
        todo!()
    }

    fn uniq(&self) -> Uniq {
        todo!()
    }

    fn local_addr(&self) -> KitsuneResult<TxUrl> {
        todo!()
    }

    fn local_cert(&self) -> Tx2Cert {
        todo!()
    }

    fn connect(&self, _url: TxUrl, _timeout: KitsuneTimeout) -> ConFut {
        todo!()
    }

    fn is_closed(&self) -> bool {
        (self.0.lock().is_closed)()
    }

    fn close(&self, _code: u32, _reason: &str) -> BoxFuture<'static, ()> {
        todo!()
    }
}

struct MockBackendAdapt(Tx2Cert, Arc<Mutex<MockFactoryHooks>>);

impl MockBackendAdapt {
    pub fn new(tls: Tx2Cert, f: MockFactoryHooks) -> AdapterFactory {
        let out: AdapterFactory = Arc::new(Self(tls, Arc::new(Mutex::new(f))));
        out
    }
}

impl BindAdapt for MockBackendAdapt {
    fn bind(&self, url: TxUrl, timeout: KitsuneTimeout) -> EndpointFut {
        let res = (self.1.lock().bind)(url, timeout);
        async move {
            let (ep, recv) = res?;
            let ep = MockEndpointAdapt(Arc::new(Mutex::new(ep)));
            let ep: Arc<dyn EndpointAdapt> = Arc::new(ep);
            let recv = MockConRecvAdapt::new(recv.stream);
            let recv: Box<dyn ConRecvAdapt> = Box::new(recv);
            Ok((ep, recv))
        }
        .boxed()
    }

    fn local_cert(&self) -> Tx2Cert {
        (self.1.lock().local_cert)(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::tls::TlsConfig;
    use crate::tx2::*;
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_combine_mock() {
        let tls = TlsConfig::new_ephemeral().await.unwrap();
        let t = KitsuneTimeout::from_millis(5000);

        let mut mem_c = MemConfig::default();
        mem_c.tls = Some(tls.clone());

        let mem_f = tx2_mem_adapter(mem_c).await.unwrap();

        let mut mock_factory = MockFactoryHooks::default();
        mock_factory.set_bind(|_url, _timeout| {
            let mut mock_endpoint = MockEndpointHooks::default();
            mock_endpoint.set_is_closed(|| true);

            Ok((mock_endpoint, Default::default()))
        });

        let mock_f = tx2_mock_adapter(tls.cert_digest.into(), mock_factory);
        let f = tx2_combine_adapter([mem_f, mock_f]);

        let (ep, _con_recv) = f.bind("none:".into(), t).await.unwrap();

        assert_eq!(false, ep.is_closed());

        assert_eq!(true, with_combine_idx(1, || ep.is_closed()));
    }
}
