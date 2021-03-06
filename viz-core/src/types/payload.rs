use bytes::{Buf, Bytes, BytesMut};

use viz_utils::futures::stream::{Stream, StreamExt};
use viz_utils::log;
use viz_utils::thiserror::Error;

use crate::http;
use crate::Context;
use crate::Response;
use crate::Result;

/// 1 MB
pub const PAYLOAD_LIMIT: usize = 1 << 20;

#[derive(Error, Debug)]
pub enum PayloadError {
    /// 400
    #[error("failed to read payload")]
    Read,

    /// 400
    #[error("failed to parse payload")]
    Parse,

    /// 411
    #[error("content-length is required")]
    LengthRequired,

    /// 413
    #[error("payload is too large")]
    TooLarge,

    /// 415
    #[error("unsupported media type")]
    UnsupportedMediaType,
}

impl Into<Response> for PayloadError {
    fn into(self) -> Response {
        (
            match self {
                Self::Read | Self::Parse => http::StatusCode::BAD_REQUEST,
                Self::LengthRequired => http::StatusCode::LENGTH_REQUIRED,
                Self::TooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                Self::UnsupportedMediaType => http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
            },
            self.to_string(),
        )
            .into()
    }
}

pub trait PayloadCheck {
    fn check_type(m: &mime::Mime) -> bool;
}

pub struct Payload<T>
where
    T: PayloadCheck,
{
    limit: usize,
    inner: Option<T>,
    check_type: Option<Box<dyn Fn(&mime::Mime) -> bool + Send + Sync>>,
}

impl<T> Payload<T>
where
    T: PayloadCheck,
{
    pub fn new(
        limit: usize,
        check_type: Option<Box<dyn Fn(&mime::Mime) -> bool + Send + Sync>>,
    ) -> Self {
        Self {
            limit,
            check_type,
            inner: None,
        }
    }

    pub fn set_check_type(
        &mut self,
        c: Box<dyn Fn(&mime::Mime) -> bool + Send + Sync>,
    ) -> &mut Self {
        self.check_type.replace(c);
        self
    }

    pub fn set_limit(&mut self, limit: usize) -> &mut Self {
        self.limit = limit;
        self
    }

    pub fn replace(&mut self, data: T) {
        self.inner.replace(data);
    }

    pub fn take(&mut self) -> T {
        self.inner.take().unwrap()
    }

    fn check_content_type(&self, m: &mime::Mime) -> bool {
        match self.check_type {
            Some(ref f) => f(m),
            None => T::check_type(m),
        }
    }

    fn check_content_length(&self, l: usize) -> bool {
        l <= self.limit
    }

    pub fn check_header(
        &self,
        m: Option<mime::Mime>,
        l: Option<usize>,
    ) -> Result<mime::Mime, PayloadError> {
        if m.is_none() {
            return Err(PayloadError::UnsupportedMediaType);
        }

        let m = m.unwrap();

        if !self.check_content_type(&m) {
            return Err(PayloadError::UnsupportedMediaType);
        }

        if l.is_none() {
            return Err(PayloadError::LengthRequired);
        }

        if !self.check_content_length(l.unwrap()) {
            return Err(PayloadError::TooLarge);
        }

        Ok(m)
    }

    pub async fn check_real_length<S>(&self, mut stream: S) -> Result<impl Buf, PayloadError>
    where
        S: Stream<Item = Result<Bytes, hyper::error::Error>> + Unpin,
    {
        let mut body = BytesMut::new();

        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|e| {
                log::debug!("{}", e);
                PayloadError::Read
            })?;
            if (body.len() + chunk.len()) > self.limit {
                return Err(PayloadError::TooLarge);
            } else {
                body.extend_from_slice(&chunk);
            }
        }

        Ok(body)
    }
}

pub fn get_mime(cx: &Context) -> Option<mime::Mime> {
    cx.header(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<mime::Mime>().ok())
}

pub fn get_length(cx: &Context) -> Option<usize> {
    cx.header(http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
}
