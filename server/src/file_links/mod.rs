//! Private browser links to a project's files: the Files view's Open Externally hands one to the
//! default browser. HTML is served as it is, Markdown as a rendered page, Excalidraw drawings as
//! an editor page that saves back.

mod pages;
mod registry;
mod serve;

pub(crate) use registry::link_path;
pub(crate) use serve::{serve, FILE_LINK_ROUTE_PREFIX};
