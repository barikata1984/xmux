use std::time::Instant;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point as TermPoint, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::TermMode;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color as TermColor, CursorShape, NamedColor};
use iced::keyboard;
use iced::widget::canvas::{self, Action, Cache, Geometry};
use iced::{Color, Event, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::Message;
use crate::input;

/// Minimum terminal grid dimensions.
const MIN_COLUMNS: u16 = 2;
const MIN_ROWS: u16 = 1;

/// Compute terminal grid size from pixel bounds and cell dimensions.
/// Returns (columns, rows) with minimum bounds enforced.
fn compute_grid_size(bounds_width: f32, bounds_height: f32, cell_width: f32, cell_height: f32) -> (u16, u16) {
    let cols = (bounds_width / cell_width).floor() as u16;
    let rows = (bounds_height / cell_height).floor() as u16;
    (cols.max(MIN_COLUMNS), rows.max(MIN_ROWS))
}

/// Default ANSI color palette (xterm-256 standard colors 0-15).
const ANSI_COLORS: [[u8; 3]; 16] = [
    [0, 0, 0],       // Black
    [205, 0, 0],     // Red
    [0, 205, 0],     // Green
    [205, 205, 0],   // Yellow
    [0, 0, 238],     // Blue
    [205, 0, 205],   // Magenta
    [0, 205, 205],   // Cyan
    [229, 229, 229], // White
    [127, 127, 127], // Bright Black
    [255, 0, 0],     // Bright Red
    [0, 255, 0],     // Bright Green
    [255, 255, 0],   // Bright Yellow
    [92, 92, 255],   // Bright Blue
    [255, 0, 255],   // Bright Magenta
    [0, 255, 255],   // Bright Cyan
    [255, 255, 255], // Bright White
];

/// Selection highlight color (semi-transparent white overlay).
const SELECTION_BG: Color = Color {
    r: 0.3,
    g: 0.5,
    b: 0.8,
    a: 0.45,
};

/// Double/triple-click threshold in milliseconds.
const CLICK_THRESHOLD_MS: u128 = 300;

pub struct TerminalView<'a> {
    pub terminal: &'a xmux_terminal::Terminal,
    pub cache: &'a Cache,
    pub cell_width: f32,
    pub cell_height: f32,
    pub pane: iced::widget::pane_grid::Pane,
    pub pane_state: &'a crate::pane::PaneState,
}

/// Widget state tracked by the canvas between frames.
#[derive(Debug)]
pub struct TerminalWidgetState {
    is_selecting: bool,
    last_click_time: Option<Instant>,
    last_click_pos: Option<Point>,
    click_count: u32,
    scroll_pixels: f32,
}

impl Default for TerminalWidgetState {
    fn default() -> Self {
        Self {
            is_selecting: false,
            last_click_time: None,
            last_click_pos: None,
            click_count: 0,
            scroll_pixels: 0.0,
        }
    }
}

/// Convert a terminal color to an iced Color, using the custom Colors table
/// for Named and Indexed variants, with hardcoded fallbacks.
fn convert_color(
    color: TermColor,
    colors: &alacritty_terminal::term::color::Colors,
) -> Color {
    match color {
        TermColor::Spec(rgb) => Color::from_rgb8(rgb.r, rgb.g, rgb.b),
        TermColor::Named(named) => {
            if let Some(rgb) = colors[named] {
                return Color::from_rgb8(rgb.r, rgb.g, rgb.b);
            }
            match named {
                NamedColor::Foreground | NamedColor::BrightForeground => {
                    Color::from_rgb8(229, 229, 229)
                }
                NamedColor::Background => Color::from_rgb8(0, 0, 0),
                NamedColor::Cursor => Color::from_rgb8(229, 229, 229),
                _ => {
                    let idx = named as usize;
                    if idx < 16 {
                        let [r, g, b] = ANSI_COLORS[idx];
                        Color::from_rgb8(r, g, b)
                    } else {
                        Color::from_rgb8(229, 229, 229)
                    }
                }
            }
        }
        TermColor::Indexed(idx) => {
            if let Some(rgb) = colors[idx as usize] {
                return Color::from_rgb8(rgb.r, rgb.g, rgb.b);
            }
            if (idx as usize) < 16 {
                let [r, g, b] = ANSI_COLORS[idx as usize];
                Color::from_rgb8(r, g, b)
            } else if idx < 232 {
                // 216-color cube (indices 16-231)
                let idx = idx - 16;
                let r = (idx / 36) * 51;
                let g = ((idx % 36) / 6) * 51;
                let b = (idx % 6) * 51;
                Color::from_rgb8(r, g, b)
            } else {
                // Grayscale ramp (indices 232-255)
                let v = 8 + (idx - 232) * 10;
                Color::from_rgb8(v, v, v)
            }
        }
    }
}

/// Convert a pixel position (relative to the canvas bounds) to a terminal grid
/// point and the side of the cell the cursor is on.
fn pixel_to_grid(
    pos: Point,
    bounds: Rectangle,
    cell_width: f32,
    cell_height: f32,
    display_offset: usize,
) -> (TermPoint, Side) {
    let rel_x = (pos.x - bounds.x).max(0.0);
    let rel_y = (pos.y - bounds.y).max(0.0);

    let col = (rel_x / cell_width) as usize;
    let row = (rel_y / cell_height) as i32;

    // Determine which side of the cell the click is on.
    let frac = rel_x / cell_width - col as f32;
    let side = if frac < 0.5 { Side::Left } else { Side::Right };

    // The display_iter yields Line(0) for the topmost visible line.
    // For scrolled views, the actual grid line is offset. However, Selection
    // works in viewport coordinates when created from renderable_content,
    // so we use Line(row) which maps to what the display_iter yields.
    // We need to account for display_offset to map to absolute grid coordinates.
    let line = Line(row - display_offset as i32);
    let point = TermPoint::new(line, Column(col));
    (point, side)
}

/// Check if two pixel positions are close enough to count as the same spot for
/// multi-click detection.
fn positions_close(a: Point, b: Point, cell_width: f32, cell_height: f32) -> bool {
    (a.x - b.x).abs() < cell_width * 2.0 && (a.y - b.y).abs() < cell_height * 2.0
}

impl<'a> canvas::Program<Message> for TerminalView<'a> {
    type State = TerminalWidgetState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        match event {
            // --- Keyboard events ---
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                text,
                modifiers,
                ..
            }) => {
                // Shift+PageUp, Shift+PageDown, Shift+Home, Shift+End -> scroll display
                if modifiers.shift() && !modifiers.control() && !modifiers.alt() {
                    if let Key::Named(named) = key {
                        match named {
                            keyboard::key::Named::PageUp => {
                                self.terminal.scroll_display(alacritty_terminal::grid::Scroll::PageUp);
                                return Some(Action::capture());
                            }
                            keyboard::key::Named::PageDown => {
                                self.terminal.scroll_display(alacritty_terminal::grid::Scroll::PageDown);
                                return Some(Action::capture());
                            }
                            keyboard::key::Named::Home => {
                                self.terminal.scroll_display(alacritty_terminal::grid::Scroll::Top);
                                return Some(Action::capture());
                            }
                            keyboard::key::Named::End => {
                                self.terminal.scroll_display(alacritty_terminal::grid::Scroll::Bottom);
                                return Some(Action::capture());
                            }
                            _ => {}
                        }
                    }
                }

                // Ctrl+B -> toggle sidebar.
                if modifiers.control() && !modifiers.shift() {
                    if let Key::Character(ch) = key {
                        match ch.as_str() {
                            "b" => {
                                return Some(Action::publish(Message::ToggleSidebar));
                            }
                            _ => {}
                        }
                    }
                }

                // Ctrl+Shift+C -> copy selection to clipboard.
                // Ctrl+Shift+V -> paste from clipboard.
                // Ctrl+Shift+D -> split pane vertically (left/right).
                // Ctrl+Shift+E -> split pane horizontally (top/bottom).
                // Ctrl+Shift+W -> close pane.
                // Ctrl+Shift+T -> new workspace.
                // Ctrl+Shift+N -> next workspace.
                // Ctrl+Shift+P -> previous workspace.
                // Ctrl+Shift+I -> inject test notification.
                if modifiers.control() && modifiers.shift() {
                    if let Key::Character(ch) = key {
                        match ch.as_str() {
                            "c" => {
                                let selected =
                                    self.terminal.with_term(|t| t.selection_to_string());
                                if let Some(text) = selected {
                                    return Some(Action::publish(Message::Copy(text)));
                                }
                                return Some(Action::capture());
                            }
                            "v" => {
                                return Some(Action::publish(Message::Paste));
                            }
                            "d" => {
                                return Some(Action::publish(Message::Split(
                                    iced::widget::pane_grid::Axis::Vertical,
                                    self.pane,
                                )));
                            }
                            "e" => {
                                return Some(Action::publish(Message::Split(
                                    iced::widget::pane_grid::Axis::Horizontal,
                                    self.pane,
                                )));
                            }
                            "w" => {
                                return Some(Action::publish(Message::ClosePane(self.pane)));
                            }
                            "t" => {
                                return Some(Action::publish(Message::NewWorkspace));
                            }
                            "n" => {
                                return Some(Action::publish(Message::NextWorkspace));
                            }
                            "p" => {
                                return Some(Action::publish(Message::PrevWorkspace));
                            }
                            "i" => {
                                return Some(Action::publish(Message::InjectTestNotification));
                            }
                            _ => {}
                        }
                    }
                }

                let is_app_cursor = self
                    .terminal
                    .with_term(|t| t.mode().contains(TermMode::APP_CURSOR));
                let text_str = text.as_ref().map(|s| s.as_str());
                if let Some(bytes) = input::key_to_bytes(key, text_str, modifiers, is_app_cursor) {
                    self.terminal.write(bytes);

                    // Auto-scroll to bottom when user types (if scrolled up).
                    let display_offset =
                        self.terminal.with_term(|t| t.grid().display_offset());
                    if display_offset > 0 {
                        self.terminal.scroll_display(alacritty_terminal::grid::Scroll::Bottom);
                    }

                    return Some(Action::capture());
                }
                None
            }

            // --- Mouse button pressed (start selection) ---
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = cursor.position_in(bounds)?;
                let display_offset =
                    self.terminal.with_term(|t| t.grid().display_offset());
                let (point, side) =
                    pixel_to_grid(pos, bounds, self.cell_width, self.cell_height, display_offset);

                // Determine click count for multi-click.
                let now = Instant::now();
                let click_count = match (state.last_click_time, state.last_click_pos) {
                    (Some(last_time), Some(last_pos))
                        if now.duration_since(last_time).as_millis() < CLICK_THRESHOLD_MS
                            && positions_close(
                                pos,
                                last_pos,
                                self.cell_width,
                                self.cell_height,
                            ) =>
                    {
                        (state.click_count % 3) + 1
                    }
                    _ => 1,
                };
                state.last_click_time = Some(now);
                state.last_click_pos = Some(pos);
                state.click_count = click_count;

                let sel_type = match click_count {
                    2 => SelectionType::Semantic,
                    3 => SelectionType::Lines,
                    _ => SelectionType::Simple,
                };

                let selection = Selection::new(sel_type, point, side);
                self.terminal.with_term_mut(|t| {
                    t.selection = Some(selection);
                });

                state.is_selecting = true;
                Some(Action::capture())
            }

            // --- Mouse moved while selecting ---
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if !state.is_selecting {
                    return None;
                }
                let pos = *position;
                // Ensure position is within bounds (or clamp).
                let display_offset =
                    self.terminal.with_term(|t| t.grid().display_offset());
                let (point, side) =
                    pixel_to_grid(pos, bounds, self.cell_width, self.cell_height, display_offset);

                self.terminal.with_term_mut(|t| {
                    if let Some(ref mut sel) = t.selection {
                        sel.update(point, side);
                    }
                });

                Some(Action::capture())
            }

            // --- Mouse button released (end selection) ---
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_selecting {
                    state.is_selecting = false;
                    return Some(Action::capture());
                }
                None
            }

            // --- Mouse wheel scrolled ---
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                // Check if in alternate screen with ALTERNATE_SCROLL mode
                let alt_scroll = self.terminal.with_term(|t| {
                    t.mode().contains(TermMode::ALT_SCREEN) && t.mode().contains(TermMode::ALTERNATE_SCROLL)
                });

                match delta {
                    mouse::ScrollDelta::Lines { y, .. } => {
                        if alt_scroll {
                            // Send arrow keys instead of scrolling display
                            let count = y.abs() as usize;
                            let arrow = if *y < 0.0 { b"\x1b[B" } else { b"\x1b[A" };
                            for _ in 0..count.max(1) {
                                self.terminal.write(arrow.to_vec());
                            }
                        } else {
                            let lines = (-y * 3.0) as i32;
                            if lines != 0 {
                                self.terminal.scroll_display(alacritty_terminal::grid::Scroll::Delta(lines));
                            }
                        }
                        return Some(Action::capture());
                    }
                    mouse::ScrollDelta::Pixels { y, .. } => {
                        if !alt_scroll {
                            // Accumulate pixel scroll in state
                            state.scroll_pixels += y;
                            let lines = (state.scroll_pixels / self.cell_height) as i32;
                            if lines != 0 {
                                state.scroll_pixels -= lines as f32 * self.cell_height;
                                self.terminal.scroll_display(alacritty_terminal::grid::Scroll::Delta(-lines));
                            }
                        }
                        return Some(Action::capture());
                    }
                }
            }

            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &TerminalWidgetState,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        // Check if the pane size has changed and update terminal grid size if needed.
        let (cols, rows) = compute_grid_size(bounds.width, bounds.height, self.cell_width, self.cell_height);
        self.pane_state.update_size(cols, rows);

        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let cw = self.cell_width;
            let ch = self.cell_height;

            // Fill the entire background.
            frame.fill_rectangle(
                Point::ORIGIN,
                bounds.size(),
                Color::from_rgb8(0, 0, 0),
            );

            self.terminal.with_term(|term| {
                let content = term.renderable_content();
                let colors = content.colors;
                let cursor = content.cursor;
                let selection = content.selection;

                // Draw cells from the grid iterator.
                for indexed in content.display_iter {
                    let cell = indexed.cell;
                    let point = indexed.point;

                    let x = point.column.0 as f32 * cw;
                    let y = point.line.0 as f32 * ch;

                    let is_wide_spacer = cell.flags.contains(Flags::WIDE_CHAR_SPACER);
                    if is_wide_spacer {
                        continue;
                    }

                    let bg = convert_color(cell.bg, colors);
                    let fg = convert_color(cell.fg, colors);

                    // Check if this cell is within the selection.
                    let in_selection = selection
                        .as_ref()
                        .map(|sel| sel.contains(point))
                        .unwrap_or(false);

                    // Draw background if not the default black, or if selected.
                    let default_bg = Color::from_rgb8(0, 0, 0);
                    let draw_bg = in_selection || bg != default_bg;
                    if draw_bg {
                        let width = if cell.flags.contains(Flags::WIDE_CHAR) {
                            cw * 2.0
                        } else {
                            cw
                        };
                        let cell_bg = if in_selection { SELECTION_BG } else { bg };
                        frame.fill_rectangle(
                            Point::new(x, y),
                            Size::new(width, ch),
                            cell_bg,
                        );
                    }

                    // Draw character.
                    if cell.c != ' ' && cell.c != '\0' {
                        let text_color = if in_selection {
                            Color::WHITE
                        } else {
                            fg
                        };
                        frame.fill_text(canvas::Text {
                            content: cell.c.to_string(),
                            position: Point::new(x, y),
                            color: text_color,
                            size: Pixels(ch * 0.85),
                            font: Font::MONOSPACE,
                            ..canvas::Text::default()
                        });
                    }
                }

                // Draw cursor.
                if cursor.shape != CursorShape::Hidden {
                    let cx = cursor.point.column.0 as f32 * cw;
                    let cy = cursor.point.line.0 as f32 * ch;
                    let cursor_color = convert_color(
                        TermColor::Named(NamedColor::Cursor),
                        colors,
                    );

                    match cursor.shape {
                        CursorShape::Block => {
                            frame.fill_rectangle(
                                Point::new(cx, cy),
                                Size::new(cw, ch),
                                Color { a: 0.7, ..cursor_color },
                            );
                        }
                        CursorShape::Beam => {
                            frame.fill_rectangle(
                                Point::new(cx, cy),
                                Size::new(2.0, ch),
                                cursor_color,
                            );
                        }
                        CursorShape::Underline => {
                            frame.fill_rectangle(
                                Point::new(cx, cy + ch - 2.0),
                                Size::new(cw, 2.0),
                                cursor_color,
                            );
                        }
                        CursorShape::HollowBlock => {
                            // Draw four edges of the block.
                            let t = 1.0;
                            frame.fill_rectangle(
                                Point::new(cx, cy),
                                Size::new(cw, t),
                                cursor_color,
                            );
                            frame.fill_rectangle(
                                Point::new(cx, cy + ch - t),
                                Size::new(cw, t),
                                cursor_color,
                            );
                            frame.fill_rectangle(
                                Point::new(cx, cy),
                                Size::new(t, ch),
                                cursor_color,
                            );
                            frame.fill_rectangle(
                                Point::new(cx + cw - t, cy),
                                Size::new(t, ch),
                                cursor_color,
                            );
                        }
                        CursorShape::Hidden => {}
                    }
                }

                // Draw scrollbar if history exists
                let grid = term.grid();
                let history = grid.history_size();
                if history > 0 {
                    let total = history + grid.screen_lines();
                    let display_offset = grid.display_offset();
                    let screen_lines = grid.screen_lines();

                    let bar_width = 8.0_f32;
                    let bar_x = bounds.size().width - bar_width;

                    // Track background
                    frame.fill_rectangle(
                        Point::new(bar_x, 0.0),
                        Size::new(bar_width, bounds.size().height),
                        Color { r: 0.3, g: 0.3, b: 0.3, a: 0.2 },
                    );

                    // Thumb
                    let thumb_top = (total - display_offset - screen_lines) as f32 / total as f32;
                    let thumb_bottom = (total - display_offset) as f32 / total as f32;
                    let thumb_y = thumb_top * bounds.size().height;
                    let thumb_h = (thumb_bottom - thumb_top) * bounds.size().height;

                    frame.fill_rectangle(
                        Point::new(bar_x, thumb_y),
                        Size::new(bar_width, thumb_h.max(10.0)),
                        Color { r: 0.6, g: 0.6, b: 0.6, a: 0.5 },
                    );
                }
            });
        });
        vec![geometry]
    }
}

use iced::keyboard::Key;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_size_from_bounds() {
        // 800x600 pixels with 8.4x16.8 cell dimensions
        // Should give us 95 columns and 35 rows
        let (cols, rows) = compute_grid_size(800.0, 600.0, 8.4, 16.8);
        assert_eq!(cols, 95);  // floor(800/8.4) = 95
        assert_eq!(rows, 35);  // floor(600/16.8) = 35
    }

    #[test]
    fn test_grid_size_minimum() {
        // Very small bounds should still enforce minimum dimensions
        let (cols, rows) = compute_grid_size(1.0, 1.0, 8.4, 16.8);
        assert_eq!(cols, MIN_COLUMNS);
        assert_eq!(rows, MIN_ROWS);
    }

    #[test]
    fn test_grid_size_typical() {
        // Typical 1024x768 window with standard cell dimensions
        let (cols, rows) = compute_grid_size(1024.0, 768.0, 8.4, 16.8);
        assert!(cols >= MIN_COLUMNS);
        assert!(rows >= MIN_ROWS);
        // Rough check: should be approximately 122 cols and 45 rows
        assert!(cols >= 120 && cols <= 125);
        assert!(rows >= 43 && rows <= 47);
    }

    #[test]
    fn test_grid_size_zero_bounds() {
        // Zero bounds should not panic and should return minimum
        let (cols, rows) = compute_grid_size(0.0, 0.0, 8.4, 16.8);
        assert_eq!(cols, MIN_COLUMNS);
        assert_eq!(rows, MIN_ROWS);
    }

    #[test]
    fn test_grid_size_respects_cell_dimensions() {
        // Larger cells should result in fewer columns/rows
        let (cols1, rows1) = compute_grid_size(800.0, 600.0, 8.4, 16.8);
        let (cols2, rows2) = compute_grid_size(800.0, 600.0, 16.0, 32.0);
        assert!(cols1 > cols2);
        assert!(rows1 > rows2);
    }

    #[test]
    fn test_scroll_delta_lines_to_i32() {
        let y = -1.0_f32;
        let lines = (-y * 3.0) as i32;
        assert_eq!(lines, 3);
    }

    #[test]
    fn test_scrollbar_position_at_bottom() {
        let history = 100usize;
        let screen_lines = 40usize;
        let display_offset = 0usize;
        let total = history + screen_lines;
        let start = (total - display_offset - screen_lines) as f32 / total as f32;
        let end = (total - display_offset) as f32 / total as f32;
        assert!((start - 0.714).abs() < 0.001);
        assert_eq!(end, 1.0);
    }

    #[test]
    fn test_scrollbar_position_at_top() {
        let history = 100usize;
        let screen_lines = 40usize;
        let display_offset = 100usize;
        let total = history + screen_lines;
        let start = (total - display_offset - screen_lines) as f32 / total as f32;
        let end = (total - display_offset) as f32 / total as f32;
        assert_eq!(start, 0.0);
        assert!((end - 0.286).abs() < 0.001);
    }
}
