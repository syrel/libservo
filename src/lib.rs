#![allow(non_snake_case)]

extern crate servo;
extern crate embedder_traits;
extern crate compositing;
extern crate webvr;
extern crate servo_media;
extern crate webrender_api;
extern crate servo_geometry;

extern crate glutin;
extern crate gleam;
extern crate euclid;

extern crate boxer;

use servo::Servo;

use embedder_traits::EventLoopWaker;
use compositing::windowing::{ EmbedderMethods, WindowMethods, EmbedderCoordinates, AnimationState };
use webvr::{ VRServiceManager, VRMainThreadHeartbeat };
use webxr_api::MainThreadRegistry;
use servo_media::player::context::{ GlApi, GlContext, NativeDisplay };
use servo_geometry::DeviceIndependentPixel;

use webrender_api::units::*;
use euclid::Scale;

use glutin::dpi::PhysicalSize;
use glutin::event_loop::EventLoopProxy;
use gleam::gl::Gl;

use std::rc::Rc;

use boxer::CBox;

pub struct GlutinEventLoopWaker {
    proxy: EventLoopProxy<()>,
}

impl EventLoopWaker for GlutinEventLoopWaker {
    // Use by servo to share the "event loop waker" across threads
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(GlutinEventLoopWaker {
            proxy: self.proxy.clone(),
        })
    }
    // Called by servo when the main thread needs to wake up
    fn wake(&self) {
        self.proxy.send_event(()).unwrap();
    }
}

struct Embedder {
     proxy: EventLoopProxy<()>
}

impl EmbedderMethods for Embedder {
    fn create_event_loop_waker(&mut self) -> Box<dyn EventLoopWaker> {
        return Box::new(GlutinEventLoopWaker { proxy: self.proxy.clone() });
    }

    fn register_vr_services(&mut self, _: &mut VRServiceManager, _: &mut Vec<Box<dyn VRMainThreadHeartbeat>>) {

    }

    fn register_webxr(&mut self, _: &mut MainThreadRegistry) {

    }
}

struct Window {
    gl: Rc<dyn Gl>,
    gl_api: glutin::Api,
    /// The pixel density of the display.
    pub hidpi_factor: Scale<f32, DeviceIndependentPixel, DevicePixel>,
    /// Size of the screen.
    pub screen: DeviceIntSize,
    /// Size of the available screen space (screen without toolbars and docks).
    pub screen_avail: DeviceIntSize,
    /// Size of the native window.
    pub window: (DeviceIntSize, DeviceIntPoint),
    /// Size of the GL buffer in the window.
    pub framebuffer: DeviceIntSize,
    /// Coordinates of the document within the framebuffer.
    pub viewport: DeviceIntRect,
}

impl WindowMethods for Window {
    fn present(&self) {

    }

    fn prepare_for_composite(&self) {

    }

    fn gl(&self) -> Rc<dyn Gl> {
        return self.gl.clone();
    }

    fn get_coordinates(&self) -> EmbedderCoordinates {
        return EmbedderCoordinates {
            hidpi_factor: self.hidpi_factor,
            screen: self.screen,
            screen_avail: self.screen_avail,
            window: self.window,
            framebuffer: self.framebuffer,
            viewport: self.viewport
        }
    }

    fn set_animation_state(&self, _state: AnimationState) {

    }

    fn get_gl_context(&self) -> GlContext {
        return GlContext::Unknown;

    }

    fn get_native_display(&self) -> NativeDisplay {
        return NativeDisplay::Headless;
    }

    fn get_gl_api(&self) -> GlApi {
        return match self.gl_api {
            glutin::Api::OpenGl => GlApi::OpenGL,
            glutin::Api::OpenGlEs => GlApi::Gles2,
            glutin::Api::WebGl => GlApi::None
        }
    }
}

fn error_callback(_gl: &dyn gleam::gl::Gl, message: &str, error: gleam::gl::GLenum) {
    println!("[GL] error: {} code: {}", message, error);
}

#[no_mangle]
fn servo_init(_ptr_event_loop: *mut glutin::event_loop::EventLoop<()>, width: i32, height: i32) -> *mut Servo<Window> {
    println!("Servo version: {}", servo::config::servo_version());

    CBox::with_raw(_ptr_event_loop, |event_loop| {
        let proxy = event_loop.create_proxy();

        let context_builder = glutin::ContextBuilder::new();
        let context = context_builder.build_headless(event_loop, PhysicalSize::new(width as f64, height as f64)).unwrap();
        let context = unsafe { context.make_current().unwrap() };

        let gl: std::rc::Rc<(dyn gleam::gl::Gl + 'static)> = match context.get_api() {
            glutin::Api::OpenGl => unsafe {
                gleam::gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            glutin::Api::OpenGlEs => unsafe {
                gleam::gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            glutin::Api::WebGl => unimplemented!(),
        };

        let gl = gleam::gl::ErrorReactingGl::wrap(gl, error_callback);

        // Implements window methods, used by compositor.
        let window = Window {
            gl,
            gl_api: context.get_api(),
            hidpi_factor: Scale::new(1.0),
            screen: DeviceIntSize::new(width, height),
            screen_avail: DeviceIntSize::new(width, height),
            window: (DeviceIntSize::new(width, height), DeviceIntPoint::new(0,0)),
            framebuffer: DeviceIntSize::new(width, height),
            viewport: DeviceIntRect::new(DeviceIntPoint::new(0,0),DeviceIntSize::new(width, height))
        };

        // Implements embedder methods, used by libservo and constellation.
        let embedder = Embedder {
            proxy
        };

        let servo = servo::Servo::new(Box::new(embedder), Rc::new(window));

        CBox::into_raw(servo)
    })
}

