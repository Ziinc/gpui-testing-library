//! `render`, `screen` and `within`.
use crate::{
    node::{Node, Snapshot},
    observe,
    query::{Queries, Query},
};
use gpui::{
    AnyWindowHandle, App, AppCell, AppContext as _, BackgroundExecutor, Context, Entity,
    HeadlessAppContext, Pixels, Render, Size, TestAppContext, Window,
};
use std::rc::Rc;

/// The GPUI app a [`Screen`] drives: a `TestAppContext`, or a
/// `HeadlessAppContext` such as the one gpui-visual-test captures from.
#[derive(Clone)]
enum Host {
    Test(TestAppContext),
    Headless {
        app: Rc<AppCell>,
        executor: BackgroundExecutor,
    },
}

impl Host {
    fn update_window<R>(
        &self,
        window: AnyWindowHandle,
        update: impl FnOnce(&mut Window, &mut App) -> R,
    ) -> Option<R> {
        match self {
            Host::Test(cx) => cx
                .clone()
                .update_window(window, |_, window, cx| update(window, cx))
                .ok(),
            Host::Headless { app, .. } => app
                .borrow_mut()
                .update_window(window, |_, window, cx| update(window, cx))
                .ok(),
        }
    }

    fn update<R>(&self, update: impl FnOnce(&mut App) -> R) -> R {
        match self {
            Host::Test(cx) => cx.update(update),
            Host::Headless { app, .. } => update(&mut app.borrow_mut()),
        }
    }

    fn read<R>(&self, read: impl FnOnce(&App) -> R) -> R {
        match self {
            Host::Test(cx) => cx.read(read),
            Host::Headless { app, .. } => read(&app.borrow()),
        }
    }

    fn executor(&self) -> BackgroundExecutor {
        match self {
            Host::Test(cx) => cx.executor(),
            Host::Headless { executor, .. } => executor.clone(),
        }
    }
}

/// Queries and interactions for one window, Testing Library's `screen`.
///
/// Cheap to clone. Every query draws a fresh frame first, so results reflect
/// the latest state. Do not call it from inside `cx.update_window`, which
/// already borrows the window.
#[derive(Clone)]
pub struct Screen {
    host: Host,
    window: AnyWindowHandle,
}

impl Screen {
    /// A screen for a window you opened yourself, for example with
    /// `gpui_kit::open_window` or `cx.add_window`.
    pub fn new(cx: &TestAppContext, window: impl Into<AnyWindowHandle>) -> Self {
        Self {
            host: Host::Test(cx.clone()),
            window: window.into(),
        }
    }

    /// A screen for a window of a `HeadlessAppContext`, the app
    /// gpui-visual-test renders real pixels with, so one test can interact
    /// through the screen and then capture the same window.
    ///
    /// Everything works as on a `TestAppContext` except
    /// [`UserEvent::upload`](crate::UserEvent::upload), which needs the test
    /// platform's file prompts, and [`Self::cx`].
    pub fn headless(cx: &HeadlessAppContext, window: impl Into<AnyWindowHandle>) -> Self {
        Self {
            host: Host::Headless {
                app: cx.app.clone(),
                executor: cx.background_executor().clone(),
            },
            window: window.into(),
        }
    }

    pub fn window(&self) -> AnyWindowHandle {
        self.window
    }

    /// The test context this screen drives.
    ///
    /// Panics for a [`Self::headless`] screen; use [`Self::update_app`] and
    /// [`Self::read_app`], which work for both.
    #[track_caller]
    pub fn cx(&self) -> TestAppContext {
        match &self.host {
            Host::Test(cx) => cx.clone(),
            Host::Headless { .. } => {
                panic!("this screen drives a HeadlessAppContext, not a TestAppContext")
            }
        }
    }

    pub(crate) fn test_cx(&self) -> Option<TestAppContext> {
        match &self.host {
            Host::Test(cx) => Some(cx.clone()),
            Host::Headless { .. } => None,
        }
    }

    /// Updates the app, for either kind of context.
    pub fn update_app<R>(&self, update: impl FnOnce(&mut App) -> R) -> R {
        self.host.update(update)
    }

    /// Reads the app, for either kind of context.
    pub fn read_app<R>(&self, read: impl FnOnce(&App) -> R) -> R {
        self.host.read(read)
    }

    /// GPUI's executor for this app, whose clock tests control.
    pub fn executor(&self) -> BackgroundExecutor {
        self.host.executor()
    }

    /// Runs spawned work until nothing more can make progress.
    pub fn run_until_parked(&self) {
        self.host.executor().run_until_parked();
    }

    /// Runs `update` with the window, then completes a frame.
    #[track_caller]
    pub fn update<R>(&self, update: impl FnOnce(&mut Window, &mut App) -> R) -> R {
        self.host
            .update_window(self.window, |window, cx| {
                let result = update(window, cx);
                draw(window, cx);
                result
            })
            .expect("the test window was closed")
    }

    /// Like Testing Library's `act`: runs `update`, lets spawned work finish,
    /// then completes a frame.
    pub fn act<R>(&self, update: impl FnOnce(&mut Window, &mut App) -> R) -> R {
        let result = self.update(update);
        self.run_until_parked();
        self.redraw();
        result
    }

    /// Completes a fresh frame.
    pub fn redraw(&self) {
        self.update(|_, _| ());
    }

    /// Closes the window, like `unmount`.
    pub fn unmount(&self) {
        let _ = self
            .host
            .update_window(self.window, |window, _| window.remove_window());
        self.run_until_parked();
    }

    /// Restricts queries to the descendants of `node`.
    pub fn within(&self, node: &Node) -> Within {
        Within {
            screen: self.clone(),
            container: node.snapshot(),
        }
    }

    /// The tree of observed elements, Testing Library's `prettyDOM`.
    pub fn pretty(&self) -> String {
        self.pretty_in(None)
    }

    /// Prints [`Self::pretty`] to stderr, like `screen.debug()`.
    pub fn debug(&self) {
        eprintln!("{}", self.pretty());
    }

    /// Prints observed elements grouped by role, like `logRoles`.
    pub fn log_roles(&self) {
        eprintln!("{}", crate::pretty::roles(&self.snapshots_in(None)));
    }

    /// Simulated user interactions, Testing Library's `userEvent.setup()`.
    pub fn user(&self) -> crate::UserEvent {
        crate::UserEvent::new(self.clone())
    }

    /// Single low-level events, Testing Library's `fireEvent`.
    pub fn fire_event(&self) -> crate::FireEvent {
        crate::FireEvent::new(self.clone())
    }

    /// Waits until `callback` is ready, Testing Library's `waitFor`.
    pub async fn wait_for<T: crate::wait::Ready>(&self, callback: impl FnMut() -> T) -> T::Output {
        crate::wait::wait_for(self, crate::wait::WaitOptions::default(), callback).await
    }

    pub(crate) fn pretty_in(&self, container: Option<&Snapshot>) -> String {
        crate::pretty::tree(&self.snapshots_in(container))
    }

    pub(crate) fn snapshots_in(&self, container: Option<&Snapshot>) -> Vec<Snapshot> {
        // A closed window shows nothing, so removed nodes read as disconnected.
        let mut snapshots = self
            .host
            .update_window(self.window, |window, cx| {
                draw(window, cx);
                collect(window)
            })
            .unwrap_or_default();
        if let Some(container) = container {
            snapshots.retain(|snapshot| container.contains(snapshot));
        }
        snapshots
    }

    pub(crate) fn resolve(&self, path: &[gpui::ElementId]) -> Option<Snapshot> {
        self.snapshots_in(None)
            .into_iter()
            .find(|snapshot| snapshot.path() == path)
    }

    /// Runs `f` with the window, or returns `None` once it is closed.
    pub(crate) fn try_with_window<R>(
        &self,
        f: impl FnOnce(&mut Window, &mut App) -> R,
    ) -> Option<R> {
        self.host.update_window(self.window, f)
    }

    pub(crate) fn with_window<R>(&self, f: impl FnOnce(&mut Window, &mut App) -> R) -> R {
        self.try_with_window(f).expect("the test window was closed")
    }
}

impl Queries for Screen {
    fn screen(&self) -> &Screen {
        self
    }
    fn container(&self) -> Option<&Snapshot> {
        None
    }
}

pub(crate) fn draw(window: &mut Window, cx: &mut App) {
    window.refresh();
    window.draw(cx).clear(cx);
}

/// Observed elements from the window's last completed frame, in reading
/// order, for code that already holds the window and so cannot use a
/// [`Screen`], such as a gpui-proptest invariant or a GPUI listener.
///
/// It does not draw: call it after a frame, not while one is being built.
pub fn observed(window: &Window) -> Vec<Snapshot> {
    collect(window)
}

/// The [`observed`] elements matching `query`, with the visibility rule of
/// [`Queries::query_all`]: role queries skip hidden elements unless asked.
pub fn observed_matching(window: &Window, query: impl Query) -> Vec<Snapshot> {
    collect(window)
        .into_iter()
        .filter(|snapshot| {
            (query.includes_hidden() || snapshot.is_visible()) && query.matches(snapshot)
        })
        .collect()
}

/// Observed elements from the last frame, in reading order: top to bottom,
/// left to right, ancestors before descendants at the same origin.
fn collect(window: &Window) -> Vec<Snapshot> {
    let mut snapshots = observe::snapshots(window);
    #[cfg(feature = "gpui-kit")]
    {
        for kit in crate::kit::snapshots(window) {
            if !snapshots.iter().any(|own| own.path() == kit.path()) {
                snapshots.push(kit);
            }
        }
    }
    snapshots.sort_by_cached_key(|snapshot| {
        (
            snapshot.bounds().origin.y,
            snapshot.bounds().origin.x,
            snapshot.path().len(),
            format!("{:?}", snapshot.path()),
        )
    });
    snapshots
}

/// Queries limited to one element's descendants. Created by [`within`].
#[derive(Clone)]
pub struct Within {
    screen: Screen,
    container: Snapshot,
}

impl Queries for Within {
    fn screen(&self) -> &Screen {
        &self.screen
    }
    fn container(&self) -> Option<&Snapshot> {
        Some(&self.container)
    }
}

impl Within {
    pub fn pretty(&self) -> String {
        self.screen.pretty_in(Some(&self.container))
    }
    pub fn debug(&self) {
        eprintln!("{}", self.pretty());
    }
}

/// Restricts queries to the descendants of `node`, Testing Library's `within`.
pub fn within(node: &Node) -> Within {
    node.screen().within(node)
}

/// Options for [`render_with`].
#[derive(Clone, Debug, Default)]
pub struct RenderOptions {
    size: Option<Size<Pixels>>,
}

impl RenderOptions {
    pub fn new() -> Self {
        Self::default()
    }
    /// The window content size. Defaults to the test display's size.
    pub fn with_size(mut self, size: Size<Pixels>) -> Self {
        self.size = Some(size);
        self
    }
    pub fn size(&self) -> Option<Size<Pixels>> {
        self.size
    }
}

/// A rendered view and the screen showing it, Testing Library's render result.
/// Dereferences to [`Screen`].
pub struct RenderResult<V> {
    screen: Screen,
    view: Entity<V>,
}

impl<V: 'static> RenderResult<V> {
    pub fn new(screen: Screen, view: Entity<V>) -> Self {
        Self { screen, view }
    }
    pub fn screen(&self) -> &Screen {
        &self.screen
    }
    pub fn view(&self) -> &Entity<V> {
        &self.view
    }
    /// Reads the view's state.
    pub fn read<R>(&self, read: impl FnOnce(&V, &App) -> R) -> R {
        self.screen.read_app(|cx| read(self.view.read(cx), cx))
    }
    /// Updates the view, then completes a frame, like `rerender`.
    pub fn rerender<R>(&self, update: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R) -> R {
        self.screen
            .update(|window, cx| self.view.update(cx, |view, cx| update(view, window, cx)))
    }
}

impl<V> std::ops::Deref for RenderResult<V> {
    type Target = Screen;
    fn deref(&self) -> &Screen {
        &self.screen
    }
}

/// Opens a test window showing the view built by `build` and draws it.
///
/// ```no_run
/// # use gpui::{TestAppContext, Render, Window, Context, IntoElement, div};
/// # struct Counter;
/// # impl Render for Counter {
/// #     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement { div() }
/// # }
/// # fn test(cx: &mut TestAppContext) {
/// use gpui_testing_library::render;
/// let screen = render(cx, |_, _| Counter);
/// # }
/// ```
pub fn render<V: Render + 'static>(
    cx: &mut TestAppContext,
    build: impl FnOnce(&mut Window, &mut Context<V>) -> V,
) -> RenderResult<V> {
    render_with(cx, RenderOptions::default(), build)
}

/// [`render`] with options.
pub fn render_with<V: Render + 'static>(
    cx: &mut TestAppContext,
    options: RenderOptions,
    build: impl FnOnce(&mut Window, &mut Context<V>) -> V,
) -> RenderResult<V> {
    let mut view = None;
    let build = |window: &mut Window, cx: &mut Context<V>| {
        view = Some(cx.entity());
        build(window, cx)
    };
    let handle = match options.size {
        Some(size) => cx.open_window(size, build),
        None => cx.add_window(build),
    };
    let screen = Screen::new(cx, handle);
    screen.redraw();
    RenderResult::new(screen, view.expect("render built the view"))
}

/// Runs `update`, then lets spawned work finish, like Testing Library's `act`.
pub fn act<R>(cx: &mut TestAppContext, update: impl FnOnce(&mut App) -> R) -> R {
    let result = cx.update(update);
    cx.run_until_parked();
    result
}

/// Closes every window, like Testing Library's `cleanup`. GPUI tears down
/// each test's app automatically; call this to reuse an app within a test.
pub fn cleanup(cx: &mut TestAppContext) {
    for window in cx.windows() {
        let _ = cx.update_window(window, |_, window, _| window.remove_window());
    }
    cx.run_until_parked();
}
