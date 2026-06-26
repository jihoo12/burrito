mod gui;
mod plot;

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
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
                        .with_title("ploty + gui")
                        .with_inner_size(winit::dpi::PhysicalSize::new(1000u32, 800u32)),
                )
                .unwrap(),
        );
        let mut app = pollster::block_on(plot::App::new(
            window.clone(),
            self.plot_data.take().unwrap(),
        ));

        // Create GUI overlay
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
        let Some(state) = self.state.as_mut() else {
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
                // Always forward to GUI
                if let Some(ref mut gui) = app.gui {
                    gui.mouse_press(btn_state == ElementState::Pressed);
                }

                // Always forward button state to camera so drag start/stop stays correct.
                // Only cursor-move is gated on GUI active state.
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

            WindowEvent::RedrawRequested => {
                app.update();

                // Build GUI frame
                let size = state.window.inner_size();
                if let Some(ref mut gui) = app.gui {
                    gui.begin_frame(size.width, size.height);

                    // Demo control panel
                    let ctrl_x = 12.0;
                    let ctrl_y = 12.0;
                    gui.group("Controls", ctrl_x, ctrl_y, 220.0, 230.0, |g| {
                        g.checkbox("Show demo panel", ctrl_x + 4.0, ctrl_y + 28.0, &mut self.show_gui_demo);
                        g.slider("Wave speed", ctrl_x + 4.0, ctrl_y + 56.0, 160.0, &mut self.wave_speed, 0.0, 10.0);
                        g.checkbox("Show grid", ctrl_x + 4.0, ctrl_y + 96.0, &mut self.show_grid);
                        app.show_grid = self.show_grid;
                        g.checkbox("2D mode", ctrl_x + 4.0, ctrl_y + 126.0, &mut self.mode_2d);
                        app.mode_2d = self.mode_2d;
                        if g.button("Reset camera", ctrl_x + 4.0, ctrl_y + 156.0, 150.0, 24.0) {
                            app.camera = plot::Camera::new();
                        }
                    });

                    // Demo widget panel
                    if self.show_gui_demo {
                        let demo_x = 12.0;
                        let demo_y = 232.0;
                        gui.group("Demo Widgets", demo_x, demo_y, 220.0, 160.0, |g| {
                            g.label("Button demo:", demo_x + 4.0, demo_y + 28.0, [0.7, 0.7, 0.7, 1.0]);
                            if g.button("Click me!", demo_x + 4.0, demo_y + 46.0, 120.0, 24.0) {
                                println!("Button clicked!");
                            }
                            let mut demo_checked = true;
                            g.checkbox("Check me", demo_x + 4.0, demo_y + 82.0, &mut demo_checked);
                            let mut demo_val = 0.5;
                            g.slider("Demo", demo_x + 4.0, demo_y + 110.0, 160.0, &mut demo_val, 0.0, 1.0);
                        });
                    }
                }

                let _ = app.render();
                state.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    let n = 60;
    let range: Vec<f32> = (0..n)
        .map(|i| -5.0 + (i as f32 / (n - 1) as f32) * 10.0)
        .collect();

    // 나선 곡선: 400 샘플, u ∈ [0, 6π]
    let helix_samples: Vec<f32> = (0..=400)
        .map(|i| i as f32 / 400.0 * 6.0 * std::f32::consts::PI)
        .collect();

    // 매듭 곡선 (trefoil knot): 600 샘플, u ∈ [0, 2π]
    let knot_samples: Vec<f32> = (0..=600)
        .map(|i| i as f32 / 600.0 * 2.0 * std::f32::consts::PI)
        .collect();

    // 2D line plot samples
    let plot2d_samples: Vec<f32> = (0..=400)
        .map(|i| -6.0 + i as f32 / 400.0 * 12.0)
        .collect();

    let config = plot::PlotConfig {
        grid_size: 12.0,
        grid_divisions: 12,
        show_axis_labels: false,
        ..Default::default()
    };

    let plot_data = plot::PlotData::new()
        .with_config(config)
        // 기존 애니메이션 물결 서피스
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
        // 정적 나선 곡선
        .add_parametric_curve(
            helix_samples,
            |u| {
                let r = 3.0;
                [r * u.cos(), u * 0.3 - 3.0, r * u.sin()]
            },
            [1.0, 0.4, 0.1],
        )
        // 애니메이션 트레포일 매듭: 크기와 위상이 시간에 따라 변함
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
        // 2D line plots (matplotlib-style)
        .add_plot2d_line(
            plot2d_samples.clone(),
            |x| x.sin(),
            [0.2, 0.4, 0.8],
        )
        .add_plot2d_line(
            plot2d_samples,
            |x| (2.0 * x).cos() * 0.7,
            [0.8, 0.2, 0.2],
        );

    let event_loop = EventLoop::new().unwrap();
    let mut handler = Handler {
        plot_data: Some(plot_data),
        state: None,
        show_gui_demo: true,
        wave_speed: 5.0,
        show_grid: true,
        mode_2d: false,
    };
    event_loop.run_app(&mut handler).unwrap();
}