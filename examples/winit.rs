use anyhow::Result;
use log::{error, info, trace};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::io::Write;
use std::time::Instant;
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::window::WindowBuilder;

use vku::{VkInit, VkInitCreateInfo};

pub fn main() {
    init_logger();
    if let Err(err) = try_main() {
        err.chain().for_each(|cause| error!("{}", cause));
        std::process::exit(1);
    }
}

pub fn try_main() -> Result<()> {
    let event_loop: EventLoop<()> = EventLoopBuilder::default().build();
    let size = [800_u32, 600_u32];
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize {
            width: size[0],
            height: size[1],
        })
        .build(&event_loop)?;
    let display_handle = window.raw_display_handle();
    let window_handle = window.raw_window_handle();
    let create_info = if cfg!(debug_assertions) {
        VkInitCreateInfo::debug_vk_1_3()
    } else {
        VkInitCreateInfo::release_vk_1_3()
    };

    let vk_init = VkInit::new(&display_handle, &window_handle, size, &create_info)?;

    let mut start_time = Instant::now();

    //Polled event loop that exits on [ESC] or window close
    event_loop.run(move |new_event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match new_event {
            Event::MainEventsCleared => {
                let end_time = Instant::now();
                let delta = end_time - start_time;
                let delta_ms = delta.as_nanos() as f32 / 1000.0 / 1000.0;
                let fps = 1.0 / delta_ms * 1000.0;
                start_time = end_time;
                info!("frame ms: {delta_ms:.2}ms | fps: {fps:.0}");
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(code) = input.virtual_keycode {
                        match code {
                            VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                            _ => (),
                        }
                    }
                }
                _ => (),
            },

            Event::LoopDestroyed => {
                vk_init.destroy().unwrap();
            }

            _ => (),
        }
    });
}

fn init_logger() {
    let env = env_logger::Env::default()
        .write_style_or("RUST_LOG_STYLE", "always")
        .filter_or("RUST_LOG", "trace");

    env_logger::Builder::from_env(env)
        .target(env_logger::Target::Stderr)
        .format(|buf, record| {
            let mut style = buf.style();

            match record.level() {
                log::Level::Info => style.set_color(env_logger::fmt::Color::Green),
                log::Level::Warn => style.set_color(env_logger::fmt::Color::Yellow),
                log::Level::Error => style.set_color(env_logger::fmt::Color::Red),
                _ => style.set_color(env_logger::fmt::Color::White),
            };

            let timestamp = buf.timestamp();

            writeln!(
                buf,
                "{:<20} : {:<5} : {}",
                timestamp,
                style.value(record.level()),
                record.args()
            )
        })
        .init();
}
