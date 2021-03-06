#![deny(unsafe_code)]

use viz_utils::anyhow;

pub use anyhow::Error;

pub type Result<T = (), E = anyhow::Error> = anyhow::Result<T, E>;

mod context;
mod extract;
mod guard;
mod handler;
mod macros;
mod middleware;
mod response;
mod types;

pub use context::Context;
pub use extract::Extract;
pub use guard::{into_guard, Guard};
pub use handler::{Handler, HandlerBase, HandlerCamp, HandlerSuper, HandlerWrapper};
pub use middleware::{DynMiddleware, Middleware, Middlewares};
pub use response::Response;
pub use types::*;

pub mod http {
    pub use ::http::*;
    pub use hyper::Body;

    pub type Request<T = Body> = ::http::Request<T>;
    pub type Response<T = Body> = ::http::Response<T>;
}
