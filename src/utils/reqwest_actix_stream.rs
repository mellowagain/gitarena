/*
 * Source: https://github.com/clia/reqwest-actix-stream
 * License: BSD 2-Clause "Simplified" License
 * Author: Cris Liao
 */

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use actix_web::error::PayloadError;
use actix_web::web;
use futures::Stream;

// Implement Send + Sync when needed
pub(crate) struct PayloadStream {
    pub(crate) payload: web::Payload,
}

impl Stream for PayloadStream {
    type Item = Result<web::Bytes, io::Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>,) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.payload).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(res))) => Poll::Ready(Some(Ok(res))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(match e {
                PayloadError::Incomplete(o) => o.unwrap_or(io::Error::new(io::ErrorKind::Other, "PayloadError::Incomplete None")),
                PayloadError::EncodingCorrupted => io::Error::new(io::ErrorKind::Other, "PayloadError::EncodingCorrupted"),
                PayloadError::Overflow => io::Error::new(io::ErrorKind::Other, "PayloadError::Overflow"),
                PayloadError::UnknownLength => io::Error::new(io::ErrorKind::Other, "PayloadError::UnknownLength"),
                PayloadError::Http2Payload(e) => io::Error::new(io::ErrorKind::Other, format!("PayloadError::Http2Payload {:?}", e)),
                PayloadError::Io(e) => e,
                _ => io::Error::new(io::ErrorKind::Other, "_") // PayloadError is marked as #[non_exhaustive]
            }))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}

pub(crate) struct ResponseStream<T> where T: Stream<Item = reqwest::Result<web::Bytes>> + Unpin {
    pub(crate) stream: T,
}

impl<T> Stream for ResponseStream<T> where T: Stream<Item = reqwest::Result<web::Bytes>> + Unpin {
    type Item = Result<web::Bytes, actix_web::Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>,) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(res))) => Poll::Ready(Some(Ok(res))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(io::Error::new(io::ErrorKind::Other, format!("{:?}", e)).into()))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}
