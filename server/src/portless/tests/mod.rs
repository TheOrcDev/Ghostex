//! Portless unit tests, grouped by what they cover. Shared fixtures (project, worktree
//! and session rows, routes, listeners, service plists, assertions) live in `fixtures.rs`.
mod desired_routes;
mod fixtures;
mod listener_detection;
mod logs_and_service_inspection;
mod route_sync;
mod setup_state;
mod slugs;

use fixtures::*;

use super::*;
