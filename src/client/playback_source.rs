use std::pin::Pin;

/// An audio source for a playback stream. At its core, this is just a callback
/// that is driven by the server to generate samples.
///
/// # Example: using a callback
///
/// A callback can be used as a [PlaybackSource] via [AsPlaybackSource]:
///
/// ```no_run
/// # use pulseaudio::*;
/// # let client = Client::from_env(c"client").unwrap();
/// # let params = protocol::PlaybackStreamParams::default();
/// let callback = move |buf: &mut [u8]| {
///     // Here, we're just returning silence.
///     buf.fill(0);
///     // We have to return the number of bytes writen, which can be less than
///     // the buffer size. However, if we return 0 bytes, that's considered an
///     // EOF, and the callback won't be called anymore.
///     buf.len()
/// };
///
/// # let _ =
/// client.create_playback_stream(params, callback.as_playback_source());
/// ```
///
/// # Example: using a type that implements AsyncRead
///
/// Types that implement [futures::io::AsyncRead] can also used as a source. In
/// this case, any error will be considered EOF.
///
/// ```no_run
/// # use pulseaudio::*;
/// # use futures::TryStreamExt;
/// # let client = Client::from_env(c"client").unwrap();
/// # let params = protocol::PlaybackStreamParams::default();
/// // Here we'll create an arbitrary stream, but this could just as easily be
/// // a PCM file or network stream or something else.
/// let stream = futures::stream::iter([
///     Ok(vec![0, 0]),
///     Ok(vec![0, 0]),
///     Ok(vec![0, 0]),
///     Ok(vec![0, 0]),
/// ]);
///
/// # let _ =
/// client.create_playback_stream(params, stream.into_async_read());
/// ```
pub trait PlaybackSource: Send + 'static {
    #[allow(missing_docs)]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
        buf: &mut [u8],
    ) -> futures::task::Poll<usize>;
}

/// A trait for converting a callback into an [AudioSource].
pub trait AsPlaybackSource {
    /// Converts the callback into an [AudioSource].
    fn as_playback_source(self) -> impl PlaybackSource;
}

struct CallbackWrapper<T: FnMut(&mut [u8]) -> usize + Send + 'static>(T);

impl<T> PlaybackSource for CallbackWrapper<T>
where
    T: FnMut(&mut [u8]) -> usize + Send + 'static,
{
    fn poll_read(
        self: Pin<&mut CallbackWrapper<T>>,
        _cx: &mut futures::task::Context<'_>,
        buf: &mut [u8],
    ) -> futures::task::Poll<usize> {
        let len = unsafe {
            let pinned_closure = Pin::get_unchecked_mut(self);
            pinned_closure.0(buf)
        };

        // We don't need to worry about waking up the reactor, because the
        // closure always returns Ok(n) or Ok(0).
        futures::task::Poll::Ready(len)
    }
}

impl<T> AsPlaybackSource for T
where
    T: FnMut(&mut [u8]) -> usize + Send + 'static,
{
    fn as_playback_source(self) -> impl PlaybackSource {
        CallbackWrapper(self)
    }
}

impl<T> PlaybackSource for T
where
    T: futures::AsyncRead + Send + 'static,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
        buf: &mut [u8],
    ) -> futures::task::Poll<usize> {
        futures::AsyncRead::poll_read(self, cx, buf).map(|result| match result {
            Ok(n) => n,
            Err(_) => 0,
        })
    }
}
