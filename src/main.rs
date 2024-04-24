use std::iter::Zip;
use std::num::NonZeroU32;

use fontdue::Font;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};
use softbuffer::{Context, Surface};
use arboard::{Clipboard, ImageData};

use winit::event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{WindowBuilder, WindowButtons, WindowLevel};
use winit::dpi::PhysicalPosition;
use winit::window::CursorIcon;
use winit::window::ResizeDirection;

use rdev::*;

use std::thread;
use std::sync::{Arc,Mutex};
use std::borrow::Cow;

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

fn render_text(text: String, size: f32, font: Font) -> Pixmap {
    let mut chars: Vec<((usize, usize), Vec<u8>)> = Vec::new();
    for ele in text.chars() {
        let (metrics, bitmap) = font.rasterize(ele, size);
        chars.push(((metrics.width, metrics.height), bitmap));
    }

    let width: usize = chars.iter().map(|i| i.0.0).sum();
    let height: usize = chars.iter().map(|i| i.0.1).sum();
    let mut pixmap = Pixmap::new(width as u32, height as u32).unwrap();

    for ele in chars {
        let mut i: u32 = 0;
        for x in 0..(ele.0.0) {
            for y in 0..(ele.0.1) {
                

                i += 1;
            }
        }
    }

    pixmap
}

fn popup_clipboard(content: ClipboardContent) {
    let event_loop = EventLoopBuilder::new().with_any_thread(true).build().unwrap();
    let window = WindowBuilder::new()
        .with_enabled_buttons(WindowButtons::empty())
        .with_decorations(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_title("FONotes - ".to_owned() + (match content {
            ClipboardContent::Image(_) => "Image",
            ClipboardContent::Text(_) => "Text",
            _ => "???"
        }))
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .with_resizable(true)
        .with_visible(true)
        .with_min_inner_size(winit::dpi::LogicalSize::new(50.0, 50.0))
        // .with_resize_increments((0,0))
        .build(&event_loop).unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.set_control_flow(ControlFlow::Wait);

    let context = Context::new(&window).unwrap();
    let mut surface = Surface::new(&context, &window).unwrap();

    let mut mouse_pos = PhysicalPosition::new(0.0, 0.0);

    event_loop.run(|event, elwt| {
        match event {
            Event::Resumed => {},
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                },
                WindowEvent::MouseInput { device_id, state, button } => {
                    // dbg!((device_id, state, button));

                    let (width, height) = {
                        let size = window.inner_size();
                        (size.width as f64, size.height as f64)
                    };

                    if button == MouseButton::Left && state.is_pressed() {
                        if mouse_pos.x > width - 30.0 && mouse_pos.y < 30.0 {         // close button
                            elwt.exit();
                        } else if mouse_pos.x < 20.0 && mouse_pos.y < 20.0 {          // west north
                            window.drag_resize_window(ResizeDirection::NorthWest).unwrap();
                        } else if mouse_pos.x < 20.0 && mouse_pos.y > height - 20.0 { // west south
                            window.drag_resize_window(ResizeDirection::SouthWest).unwrap();
                        } else if mouse_pos.x > width - 20.0 && mouse_pos.y < 20.0 {  // east north
                            window.drag_resize_window(ResizeDirection::NorthEast).unwrap();
                        } else if mouse_pos.x > width - 20.0 && mouse_pos.y > height - 20.0 { // east south
                            window.drag_resize_window(ResizeDirection::SouthEast).unwrap();
                        } else if mouse_pos.y < 20.0 && mouse_pos.x < 20.0 {          // north west
                            window.drag_resize_window(ResizeDirection::NorthWest).unwrap();
                        } else if mouse_pos.y < 20.0 && mouse_pos.x > width - 20.0 {  // north east
                            window.drag_resize_window(ResizeDirection::NorthEast).unwrap();
                        } else if mouse_pos.y > height - 20.0 && mouse_pos.x < 20.0 { // south west
                            window.drag_resize_window(ResizeDirection::SouthWest).unwrap();
                        } else if mouse_pos.y > height - 20.0 && mouse_pos.x > width - 20.0 { // south east
                            window.drag_resize_window(ResizeDirection::SouthEast).unwrap();
                        } else if mouse_pos.x < 20.0 {                                // west
                            window.drag_resize_window(ResizeDirection::West).unwrap();
                        } else if mouse_pos.x > width - 20.0 {                        // east
                            window.drag_resize_window(ResizeDirection::East).unwrap();
                        } else if mouse_pos.y > height - 20.0 {                       // south
                            window.drag_resize_window(ResizeDirection::South).unwrap();
                        } else if mouse_pos.y < 20.0 {                                // north
                            window.drag_resize_window(ResizeDirection::North).unwrap();
                        } else {                                                      // else
                            window.drag_window();
                        }
                    }
                },
                WindowEvent::CursorMoved { device_id, position } => {
                    mouse_pos = position;

                    let (width, height) = {
                        let size = window.inner_size();
                        (size.width as f64, size.height as f64)
                    };

                    if position.x > width - 30.0 && position.y < 30.0 {
                        window.set_cursor_icon(CursorIcon::Pointer);
                    } else if position.x < 20.0 || position.x > width - 20.0 {
                        window.set_cursor_icon(CursorIcon::EwResize)
                    } else if position.y < 20.0 || position.y > height - 20.0 {
                        window.set_cursor_icon(CursorIcon::NsResize)
                    } else {
                        window.set_cursor_icon(CursorIcon::Pointer);
                    }
                },
                WindowEvent::RedrawRequested => {
                    let (width, height) = {                        // объявляем переменные
                        let size = window.inner_size();   // берем размер
                        (size.width, size.height)                   // засовываем размер в тупл и выводим в переменные
                    };
                    surface
                        .resize(                                      // изменение размера сурфейса
                            NonZeroU32::new(width).unwrap(),
                            NonZeroU32::new(height).unwrap(),
                        )
                        .unwrap();
    
                    let mut pixmap = Pixmap::new(width, height).unwrap(); // берем 
                    pixmap.fill(Color::WHITE);


    
                    let mut buffer = surface.buffer_mut().unwrap();
                    for index in 0..(width * height) as usize {
                        buffer[index] = pixmap.data()[index * 4 + 2] as u32
                            | (pixmap.data()[index * 4 + 1] as u32) << 8
                            | (pixmap.data()[index * 4] as u32) << 16;
                    }
    
                    buffer.present().unwrap();
                },
                _ => (),
            },
            _ => (),
        }
    }).unwrap();

    window.set_visible(false);
}

fn main() {
    let mut pressed: Vec<Key> = Vec::new();
    let mut clipboard = Arc::new(Mutex::new(Clipboard::new().unwrap()));

    listen(move |event: rdev::Event| {
        if let EventType::KeyPress(key) = event.event_type {
            if !pressed.contains(&key) {
                pressed.push(key);
            }

            if key == Key::KeyN && 
                    pressed.contains(&Key::ControlLeft) && 
                    pressed.contains(&Key::Alt) {
                let mut clipboard_now = clipboard.clone();
                thread::spawn(move || { popup_clipboard(get_clipboard(&mut clipboard_now.lock().unwrap())) });
            }
        } else if let EventType::KeyRelease(key) = event.event_type {
            if pressed.contains(&key) {
                pressed.remove(pressed.iter().position(|x| *x == key).unwrap());
            }
        }

    }).unwrap();
}
