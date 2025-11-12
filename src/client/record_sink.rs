use std::{
    collections::VecDeque,
    io,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use futures::AsyncRead;

/// An audio sink for a record stream. At its core, this is just a callback
/// that is called whenever the server sends samples for the stream.
///
/// # Example: using a callback
///
/// A callback can be used directly as a [RecordSink].
///
/// ```no_run
/// # use pulseaudio::*;
/// # let client = Client::from_env(c"client").unwrap();
/// # let params = protocol::RecordStreamParams::default();
/// let callback = move |buf: &[u8]| {
///     // Process the audio data somehow.
/// };
///
/// # let _ =
/// client.create_record_stream(params, callback);
/// ```
///
/// # Example: using RecordBuffer
///
/// You can use a [RecordBuffer] to integrate with the async ecosystem, as it
/// implements [futures::AsyncRead].
///
/// Because of the inversion of control, data must be first written to the
/// buffer as it arrives from the server, and can then be read. This entails
/// an extra copy.
///
/// ```no_run
/// # use pulseaudio::*;
/// # let client = Client::from_env(c"client").unwrap();
/// # let params = protocol::RecordStreamParams::default();
/// // The size we pass determines the maximum amount that will be buffered.
/// let mut buffer = RecordBuffer::new(usize::MAX);
///
/// # let _ =
/// client.create_record_stream(params, buffer.as_record_sink());
///
/// // Now we can read from the buffer.
/// # let mut dst = Vec::new();
/// # async {
/// use futures::io::AsyncReadExt;
/// buffer.read(&mut dst).await?;
/// # Ok::<(), std::io::Error>(())
/// # };
/// ```
pub trait RecordSink: Send + 'static {
    #[allow(missing_docs)]
    fn write(&mut self, data: &[u8]);
}

impl<T> RecordSink for T
where
    T: FnMut(&[u8]) + Send + 'static,
{
    fn write(&mut self, data: &[u8]) {
        self(data);
    }
}

/// A buffer for adapting a record stream in situations where an implementation
/// of [AsyncRead](futures::io::AsyncRead) is required.
pub struct RecordBuffer {
    inner: Arc<Mutex<InnerRecordBuffer>>,
    capacity: usize,
}

struct InnerRecordBuffer {
    buf: VecDeque<u8>,
    waker: Option<Waker>,
    eof: bool,
}

impl RecordBuffer {
    /// Create a new record buffer with the given capacity. If you created the
    /// record stream with a specific set of
    /// [Buffer Attributes](protocol::BufferAttr), the capacity should be at
    /// least equal to the `max_length` parameter. Alternatively, just pick
    /// something reasonably large.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerRecordBuffer {
                buf: VecDeque::with_capacity(capacity),
                waker: None,
                eof: false,
            })),
            capacity,
        }
    }
}

impl std::fmt::Debug for RecordBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordBuffer")
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl AsyncRead for RecordBuffer {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut inner = self.inner.lock().unwrap();
        if inner.eof {
            return Poll::Ready(Ok(0));
        }

        let (ref mut front, _) = inner.buf.as_slices();
        if front.is_empty() {
            inner.waker = match inner.waker.take() {
                Some(w) if w.will_wake(cx.waker()) => Some(w),
                _ => Some(cx.waker().clone()),
            };

            return Poll::Pending;
        }

        let n = io::Read::read(front, buf)?;
        inner.buf.drain(..n);
        Poll::Ready(Ok(n))
    }
}

/// A newtype for the Drop implementation, which sets
/// an EOF flag for the reader.
struct RecordBufferSink(Arc<Mutex<InnerRecordBuffer>>);

impl Drop for RecordBufferSink {
    fn drop(&mut self) {
        let mut inner = self.0.lock().unwrap();
        inner.eof = true;
        if let Some(w) = inner.waker.take() {
            w.wake();
        }
    }
}

impl RecordSink for RecordBufferSink {
    fn write(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        let mut inner = self.0.lock().unwrap();

        let len = inner.buf.len();
        let to_write = data.len();
        let capacity = inner.buf.capacity();

        if to_write > capacity {
            inner.buf.clear();
            inner.buf.extend(&data[..capacity]);
        } else if to_write + len > capacity {
            inner.buf.drain(..to_write.min(len));
            inner.buf.extend(data);
        } else {
            inner.buf.extend(data);
        }

        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }
}

impl RecordBuffer {
    /// Creates a type suitable for use as a [RecordSink] when creating a new
    /// [RecordStream].
    pub fn as_record_sink(&self) -> impl RecordSink {
        RecordBufferSink(self.inner.clone())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        pin::Pin,
        sync::{Arc, Mutex},
    };

    #[test]
    fn record_buffer_asyncread() {
        let mut buffer = RecordBuffer::new(10);
        let mut sink = buffer.as_record_sink();

        sink.write(&[1, 2, 3, 4, 5]);

        let mut read_buf = [0; 3];

        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        match Pin::new(&mut buffer).poll_read(&mut cx, &mut read_buf) {
            Poll::Ready(Ok(n)) => {
                assert_eq!(n, 3);
                assert_eq!(&read_buf[..n], &[1, 2, 3]);
            }
            _ => panic!("expected ready"),
        }

        match Pin::new(&mut buffer).poll_read(&mut cx, &mut read_buf) {
            Poll::Ready(Ok(n)) => {
                assert_eq!(n, 2);
                assert_eq!(&read_buf[..n], &[4, 5]);
            }
            _ => panic!("expected ready"),
        }

        match Pin::new(&mut buffer).poll_read(&mut cx, &mut read_buf) {
            Poll::Pending => (),
            _ => panic!("expected pending"),
        }

        drop(sink);

        match Pin::new(&mut buffer).poll_read(&mut cx, &mut read_buf) {
            Poll::Ready(Ok(n)) => {
                assert_eq!(n, 0);
            }
            _ => panic!("expected ready"),
        }
    }

    #[test]
    fn record_buffer_write() {
        let buffer = RecordBuffer {
            inner: Arc::new(Mutex::new(InnerRecordBuffer {
                buf: VecDeque::with_capacity(10),
                waker: None,
                eof: false,
            })),
            capacity: 10,
        };

        let mut sink = buffer.as_record_sink();

        sink.write(&[1, 2, 3, 4, 5]);

        {
            let inner = buffer.inner.lock().unwrap();
            assert_eq!(inner.buf.len(), 5);
        }

        sink.write(&[6, 7, 8, 9, 10]);

        {
            let inner = buffer.inner.lock().unwrap();
            assert_eq!(inner.buf.len(), 10);
            assert_eq!(
                inner.buf.as_slices(),
                (&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10][..], &[][..])
            );
        }

        sink.write(&[11, 12, 13]);

        {
            let mut inner = buffer.inner.lock().unwrap();
            assert_eq!(inner.buf.len(), 10);

            inner.buf.make_contiguous();
            assert_eq!(
                inner.buf.as_slices(),
                (&[4, 5, 6, 7, 8, 9, 10, 11, 12, 13][..], &[][..])
            );
        }
    }
}
