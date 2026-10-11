//! Chat visual pages: self-contained HTML pages an agent publishes with `ghostex show`, kept under
//! the gxserver state directory and served at `/visual/<id>` so a chat card can open them in a
//! browser page. `prepare` readies the page on the CLI side (local images, theme, CSP, the question
//! kit), `publish` is the authenticated route that stores it, `serve` answers the page itself.

mod answers;
mod kit;
mod prepare;
mod publish;
mod serve;
mod store;
mod theme;

pub(crate) use answers::{is_answers_route, submit as submit_answers};
pub(crate) use prepare::prepare_page;
pub(crate) use publish::{
    handle_publish_visual_page_http, PUBLISH_VISUAL_PAGE_BODY_LIMIT_BYTES,
    PUBLISH_VISUAL_PAGE_ENDPOINT,
};
pub(crate) use serve::{serve, VISUAL_PAGE_ROUTE_PREFIX};
