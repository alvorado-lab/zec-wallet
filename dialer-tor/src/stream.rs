//! The only stream type this crate can hand out.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use arti_client::DataStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A duplex byte stream carried inside a Tor circuit.
///
/// The inner `DataStream` is arti's; the field is private and the only
/// constructor is `pub(crate)`, so nothing outside this crate — and nothing
/// inside it except [`crate::TorDialer`] — can put a non-Tor socket behind
/// this type. That is the structural half of "the dialer never returns a
/// direct connection".
#[derive(Debug)]
pub struct TorStream(DataStream);

impl TorStream {
    pub(crate) fn new(inner: DataStream) -> Self {
        Self(inner)
    }
}

impl AsyncRead for TorStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_read(cx, buf)
    }
}

impl AsyncWrite for TorStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_shutdown(cx)
    }
}
