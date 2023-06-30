use snafu::{prelude::*, Snafu};
use tokio::task::JoinHandle;
use tracing::{log::SetLoggerError, subscriber, subscriber::SetGlobalDefaultError};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("Failed to setup redirection of all logs to `LogTracer`"))]
    LogTracerInitialization { source: SetLoggerError },
    #[snafu(display("Failed to set provided subscriber as global default"))]
    SetGlobalDefault { source: SetGlobalDefaultError },
}

#[derive(derive_builder::Builder)]
#[builder(pattern = "owned")]
pub struct Telemetrio {
    #[builder(setter(into), default = r#"env!("CARGO_CRATE_NAME").into()"#)]
    name: String,
    #[builder(setter(into), default = r#""info".into()"#)]
    env_filter: String,
    #[builder(setter(custom), default = "Box::new(|| Box::new(std::io::stdout()))")]
    sink: Box<dyn Fn() -> Box<dyn std::io::Write> + Send + Sync + 'static>,
}

impl Telemetrio {
    pub fn init(self) -> Result<()> {
        let Self {
            name,
            env_filter,
            sink,
        } = self;
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
        let formatting_layer = BunyanFormattingLayer::new(name, sink);
        let subscriber = Box::new(
            Registry::default()
                .with(env_filter)
                .with(JsonStorageLayer)
                .with(formatting_layer),
        );

        LogTracer::init().context(LogTracerInitializationSnafu)?;
        subscriber::set_global_default(subscriber).context(SetGlobalDefaultSnafu)?;

        Ok(())
    }
}

impl TelemetrioBuilder {
    pub fn sink<T, F>(self, sink: F) -> Self
    where
        T: std::io::Write + Send + Sync + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            sink: Some(Box::new(move || Box::new(sink()))),
            ..self
        }
    }
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder() {
        let _ = TelemetrioBuilder::default().sink(std::io::stderr).build();
    }
}
