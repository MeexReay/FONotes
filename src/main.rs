use std::collections::HashMap;
use std::iter::Zip;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::os::linux::raw::stat;
use send_wrapper::SendWrapper;
use std::sync::mpsc::channel;

use fontdue::{Font, FontSettings};
use tiny_skia::{Color, ColorU8, FillRule, Paint, Path, PathBuilder, Pixmap, PixmapPaint, PixmapRef, Rect, Stroke, Transform};
use softbuffer::{Context, Surface};
use arboard::{Clipboard, ImageData};

use winit::event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{Window, WindowBuilder, WindowButtons, WindowId, WindowLevel};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::CursorIcon;
use winit::window::ResizeDirection;
use winit::dpi::LogicalSize;

use rdev::*;

use std::thread::{self, JoinHandle};
use std::sync::{Arc,Mutex,MutexGuard};
use std::borrow::{BorrowMut, Cow};
use std::cell::{Cell, Ref};
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use core::slice::IterMut;

enum ClipboardContent<'a> {
    Image(Cow<'a, [u8]>),
    Text(String),
    None
}

fn get_clipboard(clipboard: &mut Clipboard) -> ClipboardContent {
    match clipboard.get_image() {
        Ok(i) => {
            ClipboardContent::Image(i.bytes)
        }, Err(e) => {
            match clipboard.get_text() {
                Ok(i) => {
                    ClipboardContent::Text(i)
                }, Err(e) => {
                    ClipboardContent::None
                }
            }
        }
    }
}

fn render_char(ch: char, size: f32, font: &Font, color: Color) -> Pixmap {
    let (metrics, bitmap) = font.rasterize(ch, size);

    if ch == ' ' {
        return Pixmap::new(size as u32, size as u32).unwrap()
    }

    let mut pixmap = Pixmap::new(metrics.width as u32, metrics.height as u32).unwrap();
    let data: &mut [u8] = pixmap.data_mut();

    for j in 0..(metrics.width * metrics.height) {
        data[j * 4] = color.red() as u8;
        data[j * 4 + 1] = color.green() as u8;
        data[j * 4 + 2] = color.blue() as u8;
        data[j * 4 + 3] = bitmap[j];
    }

    pixmap
}

fn render_text(text: String, size: f32, font: &Font, char_padding: usize, color: Color) -> Pixmap {
    let mut width = 0;
    let mut height = 0;
    let mut chars: Vec<Pixmap> = Vec::new();
    for ele in text.chars() {
        let rendered = render_char(ele, size, font, color);
        width += rendered.width() + char_padding as u32;
        if rendered.height() > height { height = rendered.height(); }
        chars.push(rendered);
    }

    let mut pixmap = Pixmap::new(width, height).unwrap();
    let paint = PixmapPaint::default();

    let mut x: u32 = 0;
    for ele in chars {
        pixmap.draw_pixmap(x as i32, 0, ele.as_ref(), &paint, Transform::identity(), None);
        x += ele.width() + char_padding as u32;
    }

    pixmap
}

pub trait RemoveElem<T: PartialEq> {
    fn remove_elem<F>(&mut self, predicate: F) -> Option<T>
    where F: Fn(&T) -> bool;

    fn remove_value(&mut self, value: &T) -> Option<T>;
}

impl<T: PartialEq> RemoveElem<T> for Vec<T> {
    fn remove_elem<F>(&mut self, predicate: F) -> Option<T>
    where
        F: Fn(&T) -> bool,
    {
        self.iter()
            .position(predicate)
            .map(|index| self.remove(index))
    }

    fn remove_value(&mut self, value: &T) -> Option<T> {
        self.remove_elem(|e|{e == value})
    }
}

struct Note {
    window: Arc<Window>,
    window_id: WindowId,
    context: Context<Arc<Window>>,
    surface: Surface<Arc<Window>, Arc<Window>>,
    mouse_pos: PhysicalPosition<f64>
}

impl PartialEq for Note {
    fn eq(&self, other: &Self) -> bool {
        self.window_id == other.window_id
    }
}

impl Note {
    fn new(window: Window) -> Self {
        let arc_window = Arc::new(window);

        let context = Context::new(arc_window.clone()).unwrap();
        let mut surface = Surface::new(&context, arc_window.clone()).unwrap();
    
        let mouse_pos: PhysicalPosition<f64> = PhysicalPosition::new(0.0, 0.0);

        let window_id = arc_window.clone().id();

        Note {
            window: arc_window.clone(),
            window_id,
            context,
            surface,
            mouse_pos
        }
    }
}

fn create_event_loop() -> EventLoop<WindowBuilder> {
    let event_loop: EventLoop<WindowBuilder> = EventLoopBuilder::with_user_event().with_any_thread(true).build().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.set_control_flow(ControlFlow::Wait);

    event_loop
}

fn get_window<'a>(mut windows: IterMut<'a, Note>, id: WindowId) -> Option<&'a mut Note> {
    for note in windows {
        if note.window_id == id {
            return Some(note);
        }
    }

    None
}

fn run_event_loop<'a>(event_loop: EventLoop<WindowBuilder>, windows: RefCell<Vec<Note>>) {
    let font: Font = Font::from_bytes(include_bytes!("../resources/Roboto.ttf") as &[u8], FontSettings::default()).unwrap();

    event_loop.run(move |event, elwt| {
        let mut windows_local = windows.borrow_mut();

        match event {
            Event::Resumed => {},
            Event::UserEvent(win) => {
                let mut built = win.build(&elwt).unwrap();
                let mut win = Note::new(built);
                windows_local.push(win);
            },
            Event::WindowEvent { window_id, event } => {
                let mut win = match get_window(windows_local.iter_mut(), window_id) {
                    Some(i) => i,
                    None => {return;},
                };

                match event {
                    WindowEvent::MouseInput { device_id, state, button } => {
                        // dbg!((device_id, state, button));

                        let (width, height) = {
                            let size = win.window.inner_size();
                            (size.width as f64, size.height as f64)
                        };

                        if button == MouseButton::Left && state.is_pressed() {
                            if win.mouse_pos.x > width - 30.0 && win.mouse_pos.y < 30.0 {         // close button
                                win.window.set_visible(false);
                                windows_local.remove_elem(|e| e.window_id == window_id);
                            } else if win.mouse_pos.x < 20.0 && win.mouse_pos.y < 20.0 {          // west north
                                win.window.drag_resize_window(ResizeDirection::NorthWest).unwrap();
                            } else if win.mouse_pos.x < 20.0 && win.mouse_pos.y > height - 20.0 { // west south
                                win.window.drag_resize_window(ResizeDirection::SouthWest).unwrap();
                            } else if win.mouse_pos.x > width - 20.0 && win.mouse_pos.y < 20.0 {  // east north
                                win.window.drag_resize_window(ResizeDirection::NorthEast).unwrap();
                            } else if win.mouse_pos.x > width - 20.0 && win.mouse_pos.y > height - 20.0 { // east south
                                win.window.drag_resize_window(ResizeDirection::SouthEast).unwrap();
                            } else if win.mouse_pos.y < 20.0 && win.mouse_pos.x < 20.0 {          // north west
                                win.window.drag_resize_window(ResizeDirection::NorthWest).unwrap();
                            } else if win.mouse_pos.y < 20.0 && win.mouse_pos.x > width - 20.0 {  // north east
                                win.window.drag_resize_window(ResizeDirection::NorthEast).unwrap();
                            } else if win.mouse_pos.y > height - 20.0 && win.mouse_pos.x < 20.0 { // south west
                                win.window.drag_resize_window(ResizeDirection::SouthWest).unwrap();
                            } else if win.mouse_pos.y > height - 20.0 && win.mouse_pos.x > width - 20.0 { // south east
                                win.window.drag_resize_window(ResizeDirection::SouthEast).unwrap();
                            } else if win.mouse_pos.x < 20.0 {                                // west
                                win.window.drag_resize_window(ResizeDirection::West).unwrap();
                            } else if win.mouse_pos.x > width - 20.0 {                        // east
                                win.window.drag_resize_window(ResizeDirection::East).unwrap();
                            } else if win.mouse_pos.y > height - 20.0 {                       // south
                                win.window.drag_resize_window(ResizeDirection::South).unwrap();
                            } else if win.mouse_pos.y < 20.0 {                                // north
                                win.window.drag_resize_window(ResizeDirection::North).unwrap();
                            } else {                                                      // else
                                win.window.drag_window();
                            }
                        }
                    },
                    WindowEvent::CursorMoved { device_id, position } => {
                        win.mouse_pos = position;

                        let (width, height) = {
                            let size = win.window.inner_size();
                            (size.width as f64, size.height as f64)
                        };

                        if position.x > width - 30.0 && position.y < 30.0 {
                            win.window.set_cursor_icon(CursorIcon::Pointer);
                        } else if position.x < 20.0 || position.x > width - 20.0 {
                            win.window.set_cursor_icon(CursorIcon::EwResize)
                        } else if position.y < 20.0 || position.y > height - 20.0 {
                            win.window.set_cursor_icon(CursorIcon::NsResize)
                        } else {
                            win.window.set_cursor_icon(CursorIcon::Pointer);
                        }
                    },
                    WindowEvent::RedrawRequested => {
                        let (width, height) = {
                            let size = win.window.inner_size();
                            (size.width, size.height)
                        };
                        win.surface
                            .resize(
                                NonZeroU32::new(width).unwrap(),
                                NonZeroU32::new(height).unwrap(),
                            )
                            .unwrap();
        
                        let mut pixmap = Pixmap::new(width, height).unwrap(); // берем 
                        pixmap.fill(Color::WHITE);

                        let path = PathBuilder::from_rect(Rect::from_xywh(width as f32 - 30.0, 0.0, 30.0, 30.0).unwrap());

                        let mut paint = Paint::default();
                        paint.set_color_rgba8(220, 80, 80, 150);

                        pixmap.fill_path(
                                &path,
                                &paint,
                                FillRule::EvenOdd,
                                Transform::identity(),
                                None,
                            );

                        let mut path = PathBuilder::new();
                        path.move_to(width as f32 - 22.5, 2.5);
                        path.line_to(width as f32 - 2.5, 27.5);
                        path.line_to(width as f32 - 5.0, 27.5);
                        path.line_to(width as f32 - 25.0, 2.5);
                        path.line_to(width as f32 - 22.5, 2.5);

                        let path = path.finish().unwrap();

                        let mut paint = Paint::default();
                        paint.set_color_rgba8(220, 220, 220, 255);

                        pixmap.fill_path(
                                &path,
                                &paint,
                                FillRule::EvenOdd,
                                Transform::identity(),
                                None,
                            );
                        
                        let mut path = PathBuilder::new();

                        path.move_to(width as f32 - 5.0, 2.5);
                        path.line_to(width as f32 - 25.0, 27.5);
                        path.line_to(width as f32 - 22.5, 27.5);
                        path.line_to(width as f32 - 2.5, 2.5);
                        path.line_to(width as f32 - 5.0, 2.5);

                        let path = path.finish().unwrap();

                        pixmap.fill_path(
                                &path,
                                &paint,
                                FillRule::EvenOdd,
                                Transform::identity(),
                                None,
                            );

                        let mut paint = PixmapPaint::default();

                        pixmap.draw_pixmap(20, 20, 
                            render_text("THE QUICK BROWN FOX".to_owned(), 
                                50.0, &font, 5, Color::from_rgba8(255, 0, 0, 255)).as_ref(), 
                            &paint, Transform::identity(), None);
        
                        let mut buffer = win.surface.buffer_mut().unwrap();
                        for index in 0..(width * height) as usize {
                            buffer[index] = pixmap.data()[index * 4 + 2] as u32
                                | (pixmap.data()[index * 4 + 1] as u32) << 8
                                | (pixmap.data()[index * 4] as u32) << 16;
                        }
        
                        buffer.present().unwrap();
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }).unwrap();
}

fn popup_clipboard(content: ClipboardContent) -> WindowBuilder {
    WindowBuilder::new()
        .with_enabled_buttons(WindowButtons::empty())
        .with_decorations(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_title("FONotes - ".to_owned() + (match content {
            ClipboardContent::Image(_) => "Image",
            ClipboardContent::Text(_) => "Text",
            _ => "???"
        }))
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .with_resizable(true)
        .with_visible(true)
        .with_min_inner_size(LogicalSize::new(50.0, 50.0))
}

fn main() {
    let event_loop = create_event_loop();
    let windows = RefCell::new(Vec::new());

    let event_loop_proxy = event_loop.create_proxy();

    thread::spawn(move || {
        let mut pressed: Vec<Key> = Vec::new();
        let mut clipboard = Clipboard::new().unwrap();

        listen(move |event: rdev::Event| {
            if let EventType::KeyPress(key) = event.event_type {
                if !pressed.contains(&key) {
                    pressed.push(key);
                }

                if key == Key::KeyN && 
                        pressed.contains(&Key::ControlLeft) && 
                        pressed.contains(&Key::Alt) {
                    event_loop_proxy.send_event(popup_clipboard(get_clipboard(&mut clipboard))).unwrap();
                }
            } else if let EventType::KeyRelease(key) = event.event_type {
                if pressed.contains(&key) {
                    pressed.remove(pressed.iter().position(|x| *x == key).unwrap());
                }
            }
        }).unwrap();
    });

    run_event_loop(event_loop, windows);
}
