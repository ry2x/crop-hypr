use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use iced::{
    ContentFit, Element, Length, Point, Rectangle, Subscription, Task, Theme,
    event::listen_with,
    keyboard::{Event as KeyEvent, Key, key::Named},
    mouse,
    widget::{
        Canvas, Container, Row, Text, button, canvas,
        image::{self, Image},
        stack,
    },
};

use super::overlay::{MonitorInfo, ScreenRect, WindowInfo};

// ── Message ───────────────────────────────────────────────────────────────────

#[iced_layershell::to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum Message {
    ModeSelected(CaptureMode),
    SelectionConfirmed(ScreenRect),
    Cancel,
}

// ── Mode ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Crop,
    Window,
    Monitor,
    All,
}

// ── Canvas program (owns its data, no lifetime on AppState) ───────────────────

pub struct SelectionCanvas {
    pub mode: CaptureMode,
    pub windows: Vec<WindowInfo>,
    pub monitors: Vec<MonitorInfo>,
    /// Global pixel origin of the monitor this overlay window is on.
    /// Canvas coordinates are local (0,0 = top-left of this monitor).
    /// `canvas_local = global - offset`
    pub monitor_offset: Point,
}

// Canvas-internal mutable state
#[derive(Default)]
pub struct CanvasState {
    phase: DrawPhase,
    cursor: Point,
    hovered: Option<usize>,
}

#[derive(Default)]
enum DrawPhase {
    #[default]
    Idle,
    Cropping {
        start: Point,
    },
}

impl canvas::Program<Message> for SelectionCanvas {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut CanvasState,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let pos = cursor.position_in(bounds);

        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(p) = pos {
                    state.cursor = p;
                }
                match &state.phase {
                    DrawPhase::Cropping { .. } => {
                        return Some(canvas::Action::request_redraw());
                    }
                    DrawPhase::Idle => {
                        let prev = state.hovered;
                        state.hovered = match self.mode {
                            CaptureMode::Window => {
                                hit_index(&self.windows, pos, self.monitor_offset)
                            }
                            CaptureMode::Monitor => {
                                hit_index_m(&self.monitors, pos, self.monitor_offset)
                            }
                            _ => None,
                        };
                        if state.hovered != prev {
                            return Some(canvas::Action::request_redraw());
                        }
                    }
                }
            }

            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                match self.mode {
                    CaptureMode::Crop => {
                        if let Some(p) = pos {
                            state.phase = DrawPhase::Cropping { start: p };
                            return Some(canvas::Action::request_redraw().and_capture());
                        }
                    }
                    CaptureMode::Window => {
                        if let Some(idx) = state.hovered {
                            let rect = self.windows[idx].rect;
                            return Some(
                                canvas::Action::publish(Message::SelectionConfirmed(rect))
                                    .and_capture(),
                            );
                        }
                    }
                    CaptureMode::Monitor => {
                        if let Some(idx) = state.hovered {
                            let rect = self.monitors[idx].rect;
                            return Some(
                                canvas::Action::publish(Message::SelectionConfirmed(rect))
                                    .and_capture(),
                            );
                        }
                    }
                    CaptureMode::All => {}
                }
            }

            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let DrawPhase::Cropping { start } = state.phase {
                    state.phase = DrawPhase::Idle;
                    let local_rect = points_to_rect(start, state.cursor);
                    if local_rect.w >= 5 && local_rect.h >= 5 {
                        // Convert canvas-local coords to global for grim
                        let global_rect = ScreenRect {
                            x: local_rect.x + self.monitor_offset.x as i32,
                            y: local_rect.y + self.monitor_offset.y as i32,
                            w: local_rect.w,
                            h: local_rect.h,
                        };
                        return Some(
                            canvas::Action::publish(Message::SelectionConfirmed(global_rect))
                                .and_capture(),
                        );
                    }
                    return Some(canvas::Action::request_redraw());
                }
            }

            _ => {}
        }

        None
    }

    fn draw(
        &self,
        state: &CanvasState,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<iced::Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        frame.fill(
            &canvas::Path::rectangle(Point::ORIGIN, bounds.size()),
            iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35),
        );

        match self.mode {
            CaptureMode::Crop => {
                if let DrawPhase::Cropping { start } = state.phase {
                    draw_selection(&mut frame, start, state.cursor);
                }
            }
            CaptureMode::Window => {
                for (i, win) in self.windows.iter().enumerate() {
                    draw_highlight(
                        &mut frame,
                        win.rect,
                        state.hovered == Some(i),
                        &win.title,
                        self.monitor_offset,
                    );
                }
            }
            CaptureMode::Monitor => {
                for (i, mon) in self.monitors.iter().enumerate() {
                    draw_highlight(
                        &mut frame,
                        mon.rect,
                        state.hovered == Some(i),
                        &mon.name,
                        self.monitor_offset,
                    );
                }
            }
            CaptureMode::All => {}
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &CanvasState,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match (&state.phase, self.mode) {
            (DrawPhase::Cropping { .. }, _) | (_, CaptureMode::Crop) => {
                mouse::Interaction::Crosshair
            }
            (_, CaptureMode::Window) | (_, CaptureMode::Monitor) if state.hovered.is_some() => {
                mouse::Interaction::Pointer
            }
            _ => mouse::Interaction::default(),
        }
    }
}

// ── App State ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub mode: CaptureMode,
    /// One pre-decoded image handle per monitor (indexed same as `monitors`)
    pub monitor_images: Vec<image::Handle>,
    /// Index into `monitors` for the focused (initial) window
    pub focused_monitor_idx: usize,
    /// Maps extra window IDs (spawned at boot) → monitor index
    pub window_to_monitor: HashMap<iced::window::Id, usize>,
    pub windows: Vec<WindowInfo>,
    pub monitors: Vec<MonitorInfo>,
    /// None        = cancelled (ESC, never set)
    /// Some(None)  = "All" selected (use full screenshot)
    /// Some(Some)  = region selected
    pub result: Arc<Mutex<Option<Option<ScreenRect>>>>,
}

impl AppState {
    pub fn new(
        monitor_images: Vec<image::Handle>,
        focused_monitor_idx: usize,
        window_to_monitor: HashMap<iced::window::Id, usize>,
        windows: Vec<WindowInfo>,
        monitors: Vec<MonitorInfo>,
        result: Arc<Mutex<Option<Option<ScreenRect>>>>,
    ) -> Self {
        Self {
            mode: CaptureMode::Crop,
            monitor_images,
            focused_monitor_idx,
            window_to_monitor,
            windows,
            monitors,
            result,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ModeSelected(CaptureMode::All) => {
                *self.result.lock().unwrap() = Some(None);
                return iced::exit();
            }
            Message::ModeSelected(mode) => {
                self.mode = mode;
            }
            Message::SelectionConfirmed(rect) => {
                *self.result.lock().unwrap() = Some(Some(rect));
                return iced::exit();
            }
            Message::Cancel => {
                return iced::exit();
            }
            // Layershell control variants generated by macro — no-op
            _ => {}
        }
        Task::none()
    }

    /// Build the view for a specific window.
    /// Looks up which monitor that window is on (defaults to focused monitor)
    /// so the correct image slice and coordinate offset are used.
    pub fn view_for_window(&self, window_id: iced::window::Id) -> Element<'_, Message> {
        let mon_idx = self
            .window_to_monitor
            .get(&window_id)
            .copied()
            .unwrap_or(self.focused_monitor_idx);

        let monitor = &self.monitors[mon_idx];
        let monitor_offset = Point {
            x: monitor.rect.x as f32,
            y: monitor.rect.y as f32,
        };

        let canvas_prog = SelectionCanvas {
            mode: self.mode,
            windows: self.windows.clone(),
            monitors: self.monitors.clone(),
            monitor_offset,
        };

        let toolbar = self.toolbar();

        stack![
            Image::new(self.monitor_images[mon_idx].clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(ContentFit::Fill),
            Canvas::new(canvas_prog)
                .width(Length::Fill)
                .height(Length::Fill),
            Container::new(toolbar)
                .width(Length::Fill)
                .align_top(Length::Shrink)
                .padding(12),
        ]
        .into()
    }

    fn toolbar(&self) -> Element<'_, Message> {
        let btn = |label: &'static str, mode: CaptureMode, active: bool| {
            button(Text::new(label))
                .on_press(Message::ModeSelected(mode))
                .style(if active {
                    button::primary
                } else {
                    button::secondary
                })
                .padding([6, 14])
        };

        Container::new(
            Row::new()
                .spacing(8)
                .push(btn(
                    "✂ Crop",
                    CaptureMode::Crop,
                    self.mode == CaptureMode::Crop,
                ))
                .push(btn(
                    "🪟 Window",
                    CaptureMode::Window,
                    self.mode == CaptureMode::Window,
                ))
                .push(btn(
                    "🖥 Monitor",
                    CaptureMode::Monitor,
                    self.mode == CaptureMode::Monitor,
                ))
                .push(btn("📋 All", CaptureMode::All, false)),
        )
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.08, 0.08, 0.08, 0.85,
            ))),
            border: iced::Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .padding([6, 12])
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        listen_with(|event, _status, _id| {
            if let iced::Event::Keyboard(KeyEvent::KeyPressed {
                key: Key::Named(Named::Escape),
                ..
            }) = event
            {
                Some(Message::Cancel)
            } else {
                None
            }
        })
    }
}

// ── Drawing helpers ───────────────────────────────────────────────────────────

fn draw_selection(frame: &mut canvas::Frame, start: Point, end: Point) {
    let x = start.x.min(end.x);
    let y = start.y.min(end.y);
    let w = (start.x - end.x).abs();
    let h = (start.y - end.y).abs();

    let path = canvas::Path::rectangle(
        Point { x, y },
        iced::Size {
            width: w,
            height: h,
        },
    );
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_color(iced::Color::WHITE)
            .with_width(1.5),
    );

    frame.fill_text(canvas::Text {
        content: format!("{} × {}", w as i32, h as i32),
        position: Point {
            x: x + 4.0,
            y: (y - 20.0).max(2.0),
        },
        size: iced::Pixels(13.0),
        color: iced::Color::WHITE,
        ..canvas::Text::default()
    });
}

fn draw_highlight(
    frame: &mut canvas::Frame,
    rect: ScreenRect,
    hovered: bool,
    label: &str,
    offset: Point,
) {
    let (fill_a, stroke_a, stroke_w) = if hovered {
        (0.55f32, 1.0f32, 2.0f32)
    } else {
        (0.20, 0.7, 1.0)
    };
    // Convert global → canvas-local by subtracting monitor origin
    let x = rect.x as f32 - offset.x;
    let y = rect.y as f32 - offset.y;
    let w = rect.w as f32;
    let h = rect.h as f32;

    frame.fill_rectangle(
        Point { x, y },
        iced::Size {
            width: w,
            height: h,
        },
        iced::Color::from_rgba(0.27, 0.52, 1.0, fill_a),
    );

    let path = canvas::Path::rectangle(
        Point { x, y },
        iced::Size {
            width: w,
            height: h,
        },
    );
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_color(iced::Color::from_rgba(0.3, 0.6, 1.0, stroke_a))
            .with_width(stroke_w),
    );

    if hovered && !label.is_empty() {
        frame.fill_text(canvas::Text {
            content: label.to_owned(),
            position: Point {
                x: x + 6.0,
                y: y + 6.0,
            },
            size: iced::Pixels(13.0),
            color: iced::Color::WHITE,
            ..canvas::Text::default()
        });
    }
}

// ── Geometry helpers ──────────────────────────────────────────────────────────

fn points_to_rect(a: Point, b: Point) -> ScreenRect {
    ScreenRect {
        x: a.x.min(b.x) as i32,
        y: a.y.min(b.y) as i32,
        w: (a.x - b.x).abs() as i32,
        h: (a.y - b.y).abs() as i32,
    }
}

fn hit_index(windows: &[WindowInfo], pos: Option<Point>, offset: Point) -> Option<usize> {
    let p = pos?;
    // Convert canvas-local cursor to global for comparison with hyprctl rects
    let gx = p.x + offset.x;
    let gy = p.y + offset.y;
    windows.iter().position(|w| {
        let r = w.rect;
        gx >= r.x as f32 && gx <= (r.x + r.w) as f32 && gy >= r.y as f32 && gy <= (r.y + r.h) as f32
    })
}

fn hit_index_m(monitors: &[MonitorInfo], pos: Option<Point>, offset: Point) -> Option<usize> {
    let p = pos?;
    let gx = p.x + offset.x;
    let gy = p.y + offset.y;
    monitors.iter().position(|m| {
        let r = m.rect;
        gx >= r.x as f32 && gx <= (r.x + r.w) as f32 && gy >= r.y as f32 && gy <= (r.y + r.h) as f32
    })
}

// ── Top-level fn items (satisfy for<'a> HRTB that closures cannot) ────────────

pub fn app_update(state: &mut AppState, msg: Message) -> Task<Message> {
    state.update(msg)
}

pub fn app_view(state: &AppState, window: iced::window::Id) -> iced::Element<'_, Message> {
    state.view_for_window(window)
}

pub fn app_subscription(state: &AppState) -> Subscription<Message> {
    state.subscription()
}
