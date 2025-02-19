//! Helpers for testing debug trace calls.

use reth_primitives::H256;
use reth_rpc_api::clients::DebugApiClient;
use reth_rpc_types::trace::geth::{GethDebugTracerType, GethDebugTracingOptions};

const NOOP_TRACER: &str = include_str!("../assets/noop-tracer.js");
const JS_TRACER_TEMPLATE: &str = include_str!("../assets/tracer-template.js");

/// An extension trait for the Trace API.
#[async_trait::async_trait]
pub trait DebugApiExt {
    /// The provider type that is used to make the requests.
    type Provider;

    /// Same as [DebugApiClient::debug_trace_transaction] but returns the result as json.
    async fn debug_trace_transaction_json(
        &self,
        hash: H256,
        opts: GethDebugTracingOptions,
    ) -> Result<serde_json::Value, jsonrpsee::core::Error>;
}

#[async_trait::async_trait]
impl<T: DebugApiClient + Sync> DebugApiExt for T {
    type Provider = T;

    async fn debug_trace_transaction_json(
        &self,
        hash: H256,
        opts: GethDebugTracingOptions,
    ) -> Result<serde_json::Value, jsonrpsee::core::Error> {
        let mut params = jsonrpsee::core::params::ArrayParams::new();
        params.insert(hash).unwrap();
        params.insert(opts).unwrap();
        self.request("debug_traceTransaction", params).await
    }
}

/// A helper type that can be used to build a javascript tracer.
#[derive(Debug, Clone, Default)]
pub struct JsTracerBuilder {
    setup_body: Option<String>,
    fault_body: Option<String>,
    result_body: Option<String>,
    enter_body: Option<String>,
    step_body: Option<String>,
    exit_body: Option<String>,
}

impl JsTracerBuilder {
    /// Sets the body of the fault function
    ///
    /// The body code has access to the `log` and `db` variables.
    pub fn fault_body(mut self, body: impl Into<String>) -> Self {
        self.fault_body = Some(body.into());
        self
    }

    /// Sets the body of the setup function
    ///
    /// This body includes the `cfg` object variable
    pub fn setup_body(mut self, body: impl Into<String>) -> Self {
        self.setup_body = Some(body.into());
        self
    }

    /// Sets the body of the result function
    ///
    /// The body code has access to the `ctx` and `db` variables.
    ///
    /// ```
    /// use reth_rpc_api_testing_util::debug::JsTracerBuilder;
    /// let code = JsTracerBuilder::default().result_body("return {};").code();
    /// ```
    pub fn result_body(mut self, body: impl Into<String>) -> Self {
        self.result_body = Some(body.into());
        self
    }

    /// Sets the body of the enter function
    ///
    /// The body code has access to the `frame` variable.
    pub fn enter_body(mut self, body: impl Into<String>) -> Self {
        self.enter_body = Some(body.into());
        self
    }

    /// Sets the body of the step function
    ///
    /// The body code has access to the `log` and `db` variables.
    pub fn step_body(mut self, body: impl Into<String>) -> Self {
        self.step_body = Some(body.into());
        self
    }

    /// Sets the body of the exit function
    ///
    /// The body code has access to the `res` variable.
    pub fn exit_body(mut self, body: impl Into<String>) -> Self {
        self.exit_body = Some(body.into());
        self
    }

    /// Returns the tracers JS code
    pub fn code(self) -> String {
        let mut template = JS_TRACER_TEMPLATE.to_string();
        template = template.replace("//<setup>", self.setup_body.as_deref().unwrap_or_default());
        template = template.replace("//<fault>", self.fault_body.as_deref().unwrap_or_default());
        template =
            template.replace("//<result>", self.result_body.as_deref().unwrap_or("return {};"));
        template = template.replace("//<step>", self.step_body.as_deref().unwrap_or_default());
        template = template.replace("//<enter>", self.enter_body.as_deref().unwrap_or_default());
        template = template.replace("//<exit>", self.exit_body.as_deref().unwrap_or_default());
        template
    }
}

impl std::fmt::Display for JsTracerBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.clone().code())
    }
}

impl From<JsTracerBuilder> for GethDebugTracingOptions {
    fn from(b: JsTracerBuilder) -> Self {
        GethDebugTracingOptions {
            tracer: Some(GethDebugTracerType::JsTracer(b.code())),
            tracer_config: serde_json::Value::Object(Default::default()).into(),
            ..Default::default()
        }
    }
}
impl From<JsTracerBuilder> for Option<GethDebugTracingOptions> {
    fn from(b: JsTracerBuilder) -> Self {
        Some(b.into())
    }
}

/// A javascript tracer that does nothing
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct NoopJsTracer;

impl From<NoopJsTracer> for GethDebugTracingOptions {
    fn from(_: NoopJsTracer) -> Self {
        GethDebugTracingOptions {
            tracer: Some(GethDebugTracerType::JsTracer(NOOP_TRACER.to_string())),
            tracer_config: serde_json::Value::Object(Default::default()).into(),
            ..Default::default()
        }
    }
}
impl From<NoopJsTracer> for Option<GethDebugTracingOptions> {
    fn from(_: NoopJsTracer) -> Self {
        Some(NoopJsTracer.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        debug::{DebugApiExt, JsTracerBuilder, NoopJsTracer},
        utils::parse_env_url,
    };
    use jsonrpsee::http_client::HttpClientBuilder;

    // random tx <https://sepolia.etherscan.io/tx/0x5525c63a805df2b83c113ebcc8c7672a3b290673c4e81335b410cd9ebc64e085>
    const TX_1: &str = "0x5525c63a805df2b83c113ebcc8c7672a3b290673c4e81335b410cd9ebc64e085";

    #[tokio::test]
    #[ignore]
    async fn can_trace_noop_sepolia() {
        let tx = TX_1.parse().unwrap();
        let url = parse_env_url("RETH_RPC_TEST_NODE_URL").unwrap();
        let client = HttpClientBuilder::default().build(url).unwrap();
        let res =
            client.debug_trace_transaction_json(tx, NoopJsTracer::default().into()).await.unwrap();
        assert_eq!(res, serde_json::Value::Object(Default::default()));
    }

    #[tokio::test]
    #[ignore]
    async fn can_trace_default_template() {
        let tx = TX_1.parse().unwrap();
        let url = parse_env_url("RETH_RPC_TEST_NODE_URL").unwrap();
        let client = HttpClientBuilder::default().build(url).unwrap();
        let res = client
            .debug_trace_transaction_json(tx, JsTracerBuilder::default().into())
            .await
            .unwrap();
        assert_eq!(res, serde_json::Value::Object(Default::default()));
    }
}
