use std::sync::Arc;

use futures::FutureExt as _;

use super::ClientError;
use super::reactor::ReactorHandle;
use crate::protocol;

/// A sink for events delivered to a [Subscription]. At its core, this is just a callback
/// invoked for each event matching the subscription's mask.
///
/// # Example
///
/// ```no_run
/// # use pulseaudio::*;
/// # let client = Client::from_env(c"client").unwrap();
/// let callback = move |event: protocol::SubscriptionEvent| {
///     // Inspect event.event_facility and event.event_type here.
/// };
///
/// # let _ =
/// client.subscribe(protocol::SubscriptionMask::SERVER, callback);
/// ```
pub trait SubscriptionSink: Send + 'static {
    #[allow(missing_docs)]
    fn event(&mut self, event: protocol::SubscriptionEvent);
}

impl<T> SubscriptionSink for T
where
    T: FnMut(protocol::SubscriptionEvent) + Send + 'static,
{
    fn event(&mut self, event: protocol::SubscriptionEvent) {
        self(event)
    }
}

/// A subscription to server-side events, created with
/// [Client::subscribe](super::Client::subscribe).
///
/// PulseAudio keeps a single subscription mask per connection: creating a new
/// [Subscription] replaces the mask and sink of any previous, still-live subscription on
/// the same [Client](super::Client). Dropping the subscription stops delivery of further
/// events.
///
/// The handle can be freely cloned and shared between threads.
// The Arc is only ever inspected by `Drop`, to unsubscribe once the last clone goes away.
#[derive(Clone)]
pub struct Subscription(#[allow(dead_code)] Arc<InnerSubscription>);

struct InnerSubscription {
    handle: ReactorHandle,
}

impl std::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription").finish()
    }
}

impl Subscription {
    pub(super) async fn new(
        handle: ReactorHandle,
        mask: protocol::SubscriptionMask,
        sink: impl SubscriptionSink,
    ) -> Result<Self, ClientError> {
        handle.insert_subscription(mask, sink).await?;
        Ok(Self(Arc::new(InnerSubscription { handle })))
    }
}

impl Drop for InnerSubscription {
    fn drop(&mut self) {
        self.handle.clear_subscription();

        // Tells the server to stop sending events, but doesn't wait for the response.
        let _ = self
            .handle
            .roundtrip_ack(protocol::Command::Subscribe(
                protocol::SubscriptionMask::empty(),
            ))
            .now_or_never();
    }
}
