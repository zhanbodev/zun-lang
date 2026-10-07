use crate::error::Error;
use crate::interpreter::{HttpRoute, StaticRoute};
use crate::runtime::RuntimeContext;

/// Abstract HTTP backend trait.
/// `zun-runtime` defines routes as data; the CLI layer provides the actual
/// HTTP server implementation (currently Axum).  Swapping backends or adding
/// a new transport only requires a new `impl HttpBackend`.
pub trait HttpBackend {
    fn register_route(&mut self, route: HttpRoute);
    fn register_static(&mut self, route: StaticRoute);
    fn serve(
        self,
        context: RuntimeContext,
        host: &str,
        port: u16,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}
