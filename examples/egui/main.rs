mod graphics;

use anyhow::Result;
use egui::{
    menu, CentralPanel, ColorImage, Context, FullOutput, Key, RawInput, RichText, ScrollArea,
    TextureHandle, TextureOptions, TopBottomPanel, Visuals, Window as EguiWindow,
};
use egui_winit::State;
use graphics::Graphics;
use log::error;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::{
    error::Error,
    fs::File,
    io::{Read, Write},
    time::Instant,
};
use vku::{VkInit, VkInitCreateInfo};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

struct AppData {
    pub frame_timing_open: bool,
    pub should_close: bool,
    pub input_string: String,
    pub last_frame: Instant,
    pub frame_s: f64,
    pub start: Instant,
    pub ferris: TextureHandle,
}

pub fn main() {
    init_logger();
    if let Err(err) = try_main() {
        error!("{err}");
        std::process::exit(1);
    }
}

pub fn try_main() -> Result<(), Box<dyn Error>> {
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
    let create_info = VkInitCreateInfo::debug_vk_1_3();

    let vk_init = VkInit::new(
        Some(&display_handle),
        Some(&window_handle),
        Some(size),
        create_info,
    )?;

    let mut graphics = Graphics::new(&window)?;
    let ctx = Context::default();
    let mut state = State::new(&event_loop);

    let read_img = image::open("../../../assets/egui/textures/ferris.png").unwrap();
    let ferris = ctx.load_texture(
        "ferris",
        ColorImage::from_rgba_unmultiplied(
            [read_img.width() as usize, read_img.height() as usize],
            read_img.as_bytes(),
        ),
        TextureOptions::default(),
    );

    let mut app_data = AppData {
        frame_timing_open: true,
        should_close: false,
        input_string: String::new(),
        last_frame: Instant::now(),
        frame_s: 0.0,
        start: Instant::now(),
        ferris,
    };

    // Polled event loop that exits on [ESC] or window close
    // In a real application, you would probably encapsulate this as a method.
    event_loop.run(move |new_event, _, control_flow| {
        *control_flow = match app_data.should_close {
            true => ControlFlow::Exit,
            false => ControlFlow::Poll,
        };
        match new_event {
            Event::NewEvents(_) => {
                app_data.frame_s = app_data.last_frame.elapsed().as_secs_f64();
                app_data.last_frame = Instant::now();
            }
            Event::MainEventsCleared => {
                update(&window, &ctx, &mut state, &mut app_data, &mut graphics).unwrap();
            }
            Event::WindowEvent { event, .. } => {
                handle_window_event(
                    &ctx,
                    &mut state,
                    event,
                    &window,
                    &mut graphics,
                    &mut app_data,
                )
                .unwrap();
            }
            Event::LoopDestroyed => {
                graphics.destroy().unwrap();
                vk_init.destroy().unwrap();
            }
            _ => (),
        }
    });
}

fn handle_window_event(
    ctx: &Context,
    state: &mut State,
    event: WindowEvent,
    window: &Window,
    graphics: &mut Graphics,
    app_data: &mut AppData,
) -> Result<()> {
    if state.on_event(&ctx, &event).consumed {
        return Ok(());
    }
    match event {
        WindowEvent::Resized(new_size) => {
            //Ignore invalid resize events during startup
            if new_size == window.inner_size() && new_size.height > 0 && new_size.width > 0 {
                graphics.on_resize(new_size.into())?;
            }
        }
        WindowEvent::CloseRequested => {
            app_data.should_close = true;
        }
        WindowEvent::KeyboardInput { input, .. } => {
            if let Some(code) = input.virtual_keycode {
                match code {
                    VirtualKeyCode::Escape => app_data.should_close = true,
                    _ => (),
                }
            }
        }
        _ => (),
    }
    Ok(())
}

fn update(
    window: &Window,
    ctx: &Context,
    state: &mut State,
    app_data: &mut AppData,
    graphics: &mut Graphics,
) -> Result<()> {
    if window.inner_size().height == 0 || window.inner_size().height == 0 {
        return Ok(());
    }
    let raw_input = state.take_egui_input(window);
    let full_output = build_ui(ctx, raw_input, app_data);
    state.handle_platform_output(window, ctx, full_output.platform_output);

    let clipped_primitives = ctx.tessellate(full_output.shapes);

    let window_size = [
        window.inner_size().width as f32,
        window.inner_size().height as f32,
    ];
    let ui_to_ndc = nalgebra_glm::ortho(0.0, window_size[0], 0.0, window_size[1], -1.0, 1.0);

    graphics.update(full_output.textures_delta, clipped_primitives, ui_to_ndc)?;
    Ok(())
}

fn build_ui(ctx: &Context, raw_input: RawInput, app_data: &mut AppData) -> FullOutput {
    ctx.run(raw_input, |ctx| {
        TopBottomPanel::top("top bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                let mut style = (*ctx.style()).clone();
                if style.visuals.dark_mode {
                    if ui.button("☀").clicked() {
                        style.visuals = Visuals::light();
                        ctx.set_style(style);
                    }
                } else if ui.button("🌙").clicked() {
                    style.visuals = Visuals::dark();
                    ctx.set_style(style);
                }

                ui.separator();
                ui.menu_button("System", |ui| {
                    if ui.button("Exit application").clicked() {
                        app_data.should_close = true;
                        ui.close_menu();
                    }
                    if ui.button("Frame timing").clicked() {
                        app_data.frame_timing_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Close sub menu").clicked() {
                        ui.close_menu();
                    }
                });
            })
        });
        TopBottomPanel::bottom("bottom bar")
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Console");
                    ui.add_space(200.0);
                    let enter_command = ui.text_edit_singleline(&mut app_data.input_string);
                    if enter_command.gained_focus() {
                        app_data.input_string = String::from("");
                    }
                    if enter_command.lost_focus() && ui.input().key_pressed(Key::Enter) {
                        println!("Entered command: {}", app_data.input_string);
                        app_data.input_string = String::from("Enter command ...")
                    } else if enter_command.lost_focus() {
                        app_data.input_string = String::from("Enter command ...");
                    }
                });
                ui.separator();
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .always_show_scroll(true)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Lorem ipsum etc."));
                    });
            });

        EguiWindow::new("Frame timing")
            .open(&mut app_data.frame_timing_open)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    let frame_ms = app_data.frame_s / 1000.0;
                    let fps = 1.0 / app_data.frame_s;
                    ui.label(format!("Frame time: {frame_ms:.2}ms"));
                    ui.label(format!("FPS: {fps:.0}"));
                    ui.separator();
                    let secs_total = app_data.start.elapsed().as_secs_f64();
                    let mins = (secs_total / 60.0).floor();
                    let hours = (mins / 60.0).floor();
                    let secs = (secs_total - (hours * 60.0 * 60.0) - (mins * 60.0)).floor();

                    ui.label(format!(
                        "Time since startup: {hours:.0}h:{mins:<02.0}m:{secs:<02.0}s:"
                    ));
                });
                ui.image(
                    app_data.ferris.id(),
                    [50.0 * app_data.ferris.aspect_ratio(), 50.0],
                );
            });
        CentralPanel::default().show(ctx, |_| {});
    })
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
