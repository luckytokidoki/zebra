//! A custom middleware to trace unrecognized RPC requests.

use std::future::Future;

use futures::future::{Either, FutureExt};
use jsonrpc_core::{
    middleware::Middleware,
    types::{Call, Failure, Output, Response},
    BoxFuture, Error, ErrorCode, Metadata, MethodCall, Notification,
};

/// A custom RPC middleware that logs unrecognized RPC requests.
pub struct TracingMiddleware;

impl<M: Metadata> Middleware<M> for TracingMiddleware {
    type Future = BoxFuture<Option<Response>>;
    type CallFuture = BoxFuture<Option<Output>>;

    fn on_call<Next, NextFuture>(
        &self,
        call: Call,
        meta: M,
        next: Next,
    ) -> Either<Self::CallFuture, NextFuture>
    where
        Next: Fn(Call, M) -> NextFuture + Send + Sync,
        NextFuture: Future<Output = Option<Output>> + Send + 'static,
    {
        let call_description = self.call_description(&call);

        Either::Left(
            next(call, meta)
                .then(move |output| Self::log_error_if_method_not_found(output, call_description))
                .boxed(),
        )
    }
}

impl TracingMiddleware {
    /// Obtain a description string for a received request.
    ///
    /// Prints out only the method name and the received parameters.
    fn call_description(&self, call: &Call) -> String {
        match call {
            Call::MethodCall(MethodCall { method, params, .. }) => {
                format!(r#"method = {method:?}, params = {params:?}"#)
            }
            Call::Notification(Notification { method, params, .. }) => {
                format!(r#"notification = {method:?}, params = {params:?}"#)
            }
            Call::Invalid { .. } => "invalid request".to_owned(),
        }
    }

    /// Check an RPC output and log an error if it indicates the method was not found.
    async fn log_error_if_method_not_found(
        output: Option<Output>,
        call_description: String,
    ) -> Option<Output> {
        if let Some(Output::Failure(Failure {
            error:
                Error {
                    code: ErrorCode::MethodNotFound,
                    ..
                },
            ..
        })) = output
        {
            tracing::error!("Received unrecognized RPC request: {call_description}");
        }

        output
    }
}
