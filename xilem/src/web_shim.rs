// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Stand-ins for the `masonry_winit` types that the public API names,
//! for targets where `masonry_winit` does not build (the browser).

use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

/// Identifier of a window. On the web there is one window: the mounted canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub(crate) NonZeroU64);

static WINDOW_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl WindowId {
    /// Allocates a new, unique window id.
    pub fn next() -> Self {
        let id = WINDOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(NonZeroU64::new(id).unwrap())
    }
}

/// The winit event loop type, kept so that example `run` functions have the same
/// signature on every target. On the web it is never built.
pub type EventLoop = winit::event_loop::EventLoop<()>;
/// See [`EventLoop`].
pub type EventLoopBuilder = winit::event_loop::EventLoopBuilder<()>;
