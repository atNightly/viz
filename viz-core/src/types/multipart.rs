pub use form_data::FormData;

use viz_utils::futures::future::BoxFuture;

use crate::get_length;
use crate::get_mime;
use crate::http;
use crate::Context;
use crate::Extract;
use crate::Payload;
use crate::PayloadCheck;
use crate::PayloadError;
use crate::PAYLOAD_LIMIT;

pub type Multipart<T = http::Body> = FormData<T>;

impl PayloadCheck for Multipart {
    fn check_type(m: &mime::Mime) -> bool {
        is_multipart(m)
    }
}

impl Extract for Multipart {
    type Error = PayloadError;

    #[inline]
    fn extract<'a>(cx: &'a mut Context) -> BoxFuture<'a, Result<Self, Self::Error>> {
        Box::pin(async move {
            let payload = multipart();

            // @TODO: read context's limits config
            // payload.set_limit(limit);

            let m = get_mime(cx);
            let l = get_length(cx);

            let m = payload.check_header(m, l)?;

            let boundary = m.get_param(mime::BOUNDARY);

            if boundary.is_none() {
                return Err(PayloadError::Parse);
            }

            let boundary = boundary.unwrap().as_str();

            // let charset = m.get_param(mime::CHARSET).map(|c| c.to_string());

            Ok(Multipart::new(boundary, cx.take_body().unwrap()))
        })
    }
}

pub fn multipart() -> Payload<Multipart> {
    // Limit 16 MB
    Payload::new(PAYLOAD_LIMIT * 16, None)
}

fn is_multipart(m: &mime::Mime) -> bool {
    m.type_() == mime::MULTIPART && m.subtype() == mime::FORM_DATA
}
