mod gui;
mod plot;

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

struct State {
    window: Arc<Window>,
    app: plot::App<'static>,
}

struct Handler {
    plot_data: Option<plot::PlotData>,
    state: Option<State>,
    show_gui_demo: bool,
    wave_speed: f32,
    show_grid: bool,
    mode_2d: bool,
    text_area_content: String,
}

fn build_demo_data() -> plot::PlotData {
    let n = 60;
    let range: Vec<f32> = (0..n)
        .map(|i| -5.0 + (i as f32 / (n - 1) as f32) * 10.0)
        .collect();
    let helix_samples: Vec<f32> = (0..=400)
        .map(|i| i as f32 / 400.0 * 6.0 * std::f32::consts::PI)
        .collect();
    let knot_samples: Vec<f32> = (0..=600)
        .map(|i| i as f32 / 600.0 * 2.0 * std::f32::consts::PI)
        .collect();
    let plot2d_samples: Vec<f32> = (0..=400)
        .map(|i| -6.0 + i as f32 / 400.0 * 12.0)
        .collect();

    let bar_labels: Vec<f32> = (0..7).map(|i| i as f32 - 3.0).collect();
    let bar_heights: Vec<f32> = vec![1.2, 3.4, 2.1, 4.5, 3.0, 1.8, 2.7];

    let config = plot::PlotConfig {
        grid_size: 12.0,
        grid_divisions: 12,
        show_axis_labels: false,
        ..Default::default()
    };

    plot::PlotData::new()
        .with_config(config)
        .add_animated_graph(
            range.clone(),
            range.clone(),
            |x, z, t| {
                let r = (x * x + z * z).sqrt();
                let width = 2.0;
                let gaussian = (-(r * r) / (2.0 * width * width)).exp();
                let wave = (4.0 * r - t * 5.0).cos();
                gaussian * wave
            },
            [0.1, 0.8, 0.4],
        )
        .add_parametric_curve(
            helix_samples,
            |u| {
                let r = 3.0;
                [r * u.cos(), u * 0.3 - 3.0, r * u.sin()]
            },
            [1.0, 0.4, 0.1],
        )
        .add_animated_parametric_curve(
            knot_samples,
            |u, t| {
                let scale = 2.5 + 0.5 * (t * 0.8).sin();
                let x = scale * (u.sin() + 2.0 * (2.0 * u).sin());
                let y = scale * 0.4 * (t * 0.5 + u).cos();
                let z = scale * (u.cos() - 2.0 * (2.0 * u).cos());
                [x, y, z]
            },
            [0.3, 0.6, 1.0],
        )
        .add_plot2d_line(
            plot2d_samples.clone(),
            |x| x.sin(),
            [0.2, 0.4, 0.8],
        )
        .add_plot2d_line(
            plot2d_samples,
            |x| (2.0 * x).cos() * 0.7,
            [0.8, 0.2, 0.2],
        )
        .add_plot2d_filled(plot::plot_2d_bar(&bar_labels, &bar_heights, 0.5, [0.3, 0.6, 0.9]))
        // Canvas2D demo: draw a simple triangle and circle overlay
        .add_plot2d_filled({
            let mut c = plot::Canvas2D::new();
            c.set_fill_style([0.9, 0.3, 0.3]);
            c.begin_path();
            c.move_to(0.0, 2.0);
            c.line_to(1.5, 0.5);
            c.line_to(-1.5, 0.5);
            c.close_path();
            c.fill();
            c.set_stroke_style([0.1, 0.1, 0.8]);
            c.set_line_width(3.0);
            c.begin_path();
            c.arc(3.0, 2.5, 1.0, 0.0, std::f32::consts::PI * 2.0);
            c.stroke();
            c.build()
        })
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("burrito")
                        .with_inner_size(winit::dpi::PhysicalSize::new(1000u32, 800u32)),
                )
                .unwrap(),
        );
        let mut app = pollster::block_on(plot::App::new(
            window.clone(),
            self.plot_data.take().unwrap(),
        ));

        let mut gui = gui::Gui::new(app.device(), app.queue(), app.format());
        gui.resize(1000, 800);
        app.gui = Some(gui);

        self.state = Some(State { window, app });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Handler {
            state,
            show_gui_demo,
            wave_speed,
            show_grid,
            mode_2d,
            text_area_content,
            plot_data: _,
        } = self;

        let Some(state) = state.as_mut() else {
            return;
        };
        let app = &mut state.app;

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                app.resize(size);
                if let Some(ref mut gui) = app.gui {
                    gui.resize(size.width, size.height);
                }
            }
            WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                if let Some(ref mut gui) = app.gui {
                    gui.mouse_press(btn_state == ElementState::Pressed);
                }
                match button {
                    MouseButton::Left => {
                        app.camera.on_mouse_button(btn_state == ElementState::Pressed);
                    }
                    MouseButton::Middle => {
                        app.camera.on_middle_mouse_button(btn_state == ElementState::Pressed);
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(ref mut gui) = app.gui {
                    gui.mouse_move(position.x, position.y);
                }
                let gui_active = app.gui.as_ref().map_or(false, |g| g.is_active());
                if !gui_active {
                    app.camera.on_cursor_moved(position.x, position.y);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let gui_active = app.gui.as_ref().map_or(false, |g| g.is_active());
                if !gui_active {
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.01,
                    };
                    app.camera.on_scroll(dy);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(ref mut gui) = app.gui {
                    if event.state == ElementState::Pressed {
                        let c = match &event.logical_key {
                            Key::Character(s) => s.chars().next(),
                            _ => None,
                        };
                        let backspace = matches!(&event.logical_key, Key::Named(NamedKey::Backspace));
                        let delete = matches!(&event.logical_key, Key::Named(NamedKey::Delete));
                        let enter = matches!(&event.logical_key, Key::Named(NamedKey::Enter));
                        gui.key_event(c, backspace, delete, enter);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                app.update();

                let size = state.window.inner_size();
                let mut reset_camera = false;

                if let Some(ref mut gui) = app.gui {
                    gui.begin_frame(size.width, size.height);

                    let ctrl_x = 12.0;
                    let ctrl_y = 12.0;
                    gui.group("Controls", ctrl_x, ctrl_y, 220.0, 230.0, |g| {
                        g.checkbox("Show demo panel", ctrl_x + 4.0, ctrl_y + 28.0, show_gui_demo);
                        g.slider("Wave speed", ctrl_x + 4.0, ctrl_y + 56.0, 160.0, wave_speed, 0.0, 10.0);
                        g.checkbox("Show grid", ctrl_x + 4.0, ctrl_y + 96.0, show_grid);
                        g.checkbox("2D mode", ctrl_x + 4.0, ctrl_y + 126.0, mode_2d);

                        if g.button("Reset camera", ctrl_x + 4.0, ctrl_y + 156.0, 150.0, 24.0) {
                            reset_camera = true;
                        }
                    });

                    if *show_gui_demo {
                        let demo_x = 12.0;
                        let demo_y = 232.0;
                        gui.group("Demo Widgets", demo_x, demo_y, 220.0, 260.0, |g| {
                            g.label("Button demo:", demo_x + 4.0, demo_y + 28.0, [0.7, 0.7, 0.7, 1.0]);
                            if g.button("Click me!", demo_x + 4.0, demo_y + 46.0, 120.0, 24.0) {
                                println!("Button clicked!");
                            }
                            let mut demo_checked = true;
                            g.checkbox("Check me", demo_x + 4.0, demo_y + 82.0, &mut demo_checked);
                            let mut demo_val = 0.5;
                            g.slider("Demo", demo_x + 4.0, demo_y + 110.0, 160.0, &mut demo_val, 0.0, 1.0);
                            g.label("Text area:", demo_x + 4.0, demo_y + 140.0, [0.7, 0.7, 0.7, 1.0]);
                            g.text_area(demo_x + 4.0, demo_y + 156.0, 200.0, 84.0, text_area_content);
                        });
                    }
                }

                app.show_grid = *show_grid;
                app.mode_2d = *mode_2d;
                if reset_camera {
                    app.camera = plot::Camera::new();
                }

                let _ = app.render();
                state.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut handler = Handler {
        plot_data: Some(build_demo_data()),
        state: None,
        show_gui_demo: true,
        wave_speed: 5.0,
        show_grid: true,
        mode_2d: false,
        text_area_content: String::new(),
    };
    event_loop.run_app(&mut handler).unwrap();
}
