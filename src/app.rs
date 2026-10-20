//! # App
//!
//! Handles creation of the app and window
//!
//! Also handles window events

use anyhow::Ok;
use winit::{
    application::ApplicationHandler, event::{KeyEvent, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{PhysicalKey}, window::{Window, WindowAttributes}
};
use std::sync::Arc;
use env_logger;
use crate::state::State;

/// # Runtime app and window
///
/// Handles the creation of the state of the window at runtime
pub struct App {
    /// # Proxy
    ///
    /// Sends custom events during web runtime.
    ///
    /// Fixes errors related to WGPU Async
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,

    /// # App State
    ///
    ///  Stores [`State`] as an [`Option`] variable.
    ///
    /// It's an [`Option`] because window can only be returned in the `Resumed` states.
    state: Option<State>,
}

impl App {
    /// Creates the full app
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());

        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

// Adds extra functions to App struct
impl ApplicationHandler<State> for App {
    /// # Resumed
    ///
    /// Does a few things:
    /// - Defines attributes about the window
    /// - Handle web-specific attributes (`WASM32`)
    /// - Use those attributes to create the window
    /// - Create a future that creates a `State `struct
    /// - On native:
    ///     - Use Pollster to await the future
    /// - On web:
    ///     - Run the future async
    ///     - Sends the results to the `user_event` function
    /// ## Original Docs
    ///
    /// Emitted when the application has been resumed.
    ///
    /// For consistency, all platforms emit a `Resumed` event even if they don't themselves have a formal suspend/resume lifecycle.
    /// For systems without a formal suspend/resume lifecycle the `Resumed` event is always emitted after the `NewEvents(StartCause::Init)` event.
    ///
    /// ### Portability
    /// It's recommended that applications should only initialize their graphics context and create a window after they have received their first `Resumed`
    /// event. Some systems (specifically Android) won't allow applications to create a render surface until they are resumed.
    ///
    /// Considering that the implementation of `Suspended` and `Resumed` events may be internally driven by multiple platform-specific events, and that there
    /// may be subtle differences across platforms with how these internal events are delivered, it's recommended that applications be able to gracefully handle
    /// redundant (i.e. back-to-back) `Suspended` or `Resumed` events.
    ///
    /// Also see `Suspended` notes.
    ///
    /// ### Android
    /// On Android, the `Resumed` event is sent when a new `SurfaceView` has been created. This is expected to closely correlate with the `onResume`
    /// lifecycle event but there may technically be a discrepancy.
    ///
    /// Applications that need to run on Android must wait until they have been `Resumed` before they will be able to create a render surface (such as an
    /// `EGLSurface`, `VkSurfaceKHR` or [Surface][`wgpu::Surface`]) which depend on having a `SurfaceView`. Applications must also assume that if they are
    /// `Suspended`, then their render surfaces are invalid and should be dropped.
    ///
    /// Also see `Suspended` notes.
    ///
    /// ### iOS
    /// On iOS, the `Resumed` event is emitted in response to an `applicationDidBecomeActive` callback which means the application is "active" (according to
    /// the [iOS application lifecycle](https://developer.apple.com/documentation/uikit/app_and_environment/managing_your_app_s_life_cycle)).
    ///
    /// ### Web
    /// On Web, the `Resumed` event is emitted in response to a `pageshow` event with the property `persisted` being true, which means that the page is
    /// being restored from the `bfcache` (back/forward cache) - an in-memory cache that stores a complete snapshot of a page (including the JavaScript heap)
    /// as the user is navigating away.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Allow unused mut in case it doesn't change
        #[allow(unused_mut)]
        let mut window_attributes: WindowAttributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window: Arc<Window> = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we are not on web, use pollster to await the window creation
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            // Create the window
                            State::new(window)
                                // Wait for it to be sent
                                .await
                                // Throw an error if it never sends
                                .expect("Unable to create canvas!!!")
                        )
                        .is_ok())
                });
            }
        }
    }

    /// # User Event
    ///
    /// Serves as a landing point for the `State` future because [`App::resumed`] isn't async.
    /// Offloads the future and send the results somewhere
    ///
    /// ## Original Docs
    ///
    /// Emitted when an event is sent from EventLoopProxy::send_event.
    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        // This is where proxy.send_event() ends up
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Copies current state
        let state: &mut State = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        // Checks what the most recent event was and performs an action
        // Similar a Java Switch-Case, with slightly minor differences
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    // If output succededs, continue
                    core::result::Result::Ok(_) => {}
                    // Else quit the app
                    core::result::Result::Err(e) => {
                        // Log the error and exit gracefully
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            },
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        // Fills in the rest needed
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            // Everything else
            _ => ()
        }
    }
}

/// # Run
///
/// Runs the window initialization for both WASM and other architectures
///
/// Creates the app variable and runs it through event_loop
pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }

    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop: EventLoop<_> = EventLoop::with_user_event().build()?;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new();
        event_loop.run_app(&mut app)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let app = App::new(&event_loop);
        event_loop.spawn_app(app);
    }

    Ok(())
}

/// # Run Web
///
/// Handles running the application on the web
///
/// Manages extra console requirements so code panics can be viewed in the console
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}
