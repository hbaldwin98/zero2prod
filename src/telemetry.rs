use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{
    fmt::MakeWriter, prelude::__tracing_subscriber_SubscriberExt, EnvFilter, Registry,
};

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}
