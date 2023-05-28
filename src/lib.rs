use tokio::task::JoinHandle;
use tracing::{dispatcher::SetGlobalDefaultError, log::SetLoggerError, subscriber, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to set logger")]
    LogTracerInit(#[from] SetLoggerError),
    #[error("Failed to set global default subscriber")]
    SetGlobalDefault(#[from] SetGlobalDefaultError),
}

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to spell out the actual
/// type of the returned subscriber, which is indeed quite complex.
pub fn build_subscriber<Sink>(
    name: &str,
    env_filter: &str,
    sink: Sink,
) -> Box<dyn Subscriber + Send + Sync>
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name.to_owned(), sink);
    Box::new(
        Registry::default()
            .with(env_filter)
            .with(JsonStorageLayer)
            .with(formatting_layer),
    )
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) -> Result<(), Error> {
    LogTracer::init().map_err(Into::<Error>::into)?;
    subscriber::set_global_default(subscriber).map_err(Into::<Error>::into)?;
    Ok(())
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}
