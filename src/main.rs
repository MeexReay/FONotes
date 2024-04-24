use std::num::NonZeroU32;

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};
use softbuffer::{Context, Surface};
use arboard::{Clipboard, ImageData};

use winit::event::{Event, WindowEvent, KeyEvent, ElementState};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{WindowBuilder, WindowButtons, WindowLevel};

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

fn popup_clipboard(content: ClipboardContent) {
    let event_loop = EventLoopBuilder::new().with_any_thread(true).build().unwrap();
    let window = WindowBuilder::new()
        .with_enabled_buttons(WindowButtons::empty())
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_decorations(false)
        .with_title("My Window")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .with_resizable(true)
        .with_visible(true)
        .build(&event_loop).unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.set_control_flow(ControlFlow::Wait);

    let context = Context::new(&window).unwrap();
    let mut surface = Surface::new(&context, &window).unwrap();

    event_loop.run(|event, elwt| {
        match event {
            Event::Resumed => {
                dbg!("what");
            },
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                },
                WindowEvent::MouseInput { device_id, state, button } => {
                    dbg!((device_id, state, button));
                },
                WindowEvent::CursorMoved { device_id, position } => {
                    dbg!((device_id, position));
                },
                WindowEvent::CursorEntered { device_id } => {
                    println!("cursor entered in the window {window_id:?}");
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
