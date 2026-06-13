use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color as TermColor, CursorShape, NamedColor};
use iced::widget::canvas::{self, Cache, Geometry};
use iced::{Color, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::Message;

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

pub struct TerminalView<'a> {
    pub terminal: &'a xmux_terminal::Terminal,
    pub cache: &'a Cache,
    pub cell_width: f32,
    pub cell_height: f32,
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

impl<'a> canvas::Program<Message> for TerminalView<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
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

                    // Draw background if not the default black.
                    let default_bg = Color::from_rgb8(0, 0, 0);
                    if bg != default_bg {
                        let width = if cell.flags.contains(Flags::WIDE_CHAR) {
                            cw * 2.0
                        } else {
                            cw
                        };
                        frame.fill_rectangle(
                            Point::new(x, y),
                            Size::new(width, ch),
                            bg,
                        );
                    }

                    // Draw character.
                    if cell.c != ' ' && cell.c != '\0' {
                        frame.fill_text(canvas::Text {
                            content: cell.c.to_string(),
                            position: Point::new(x, y),
                            color: fg,
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
            });
        });
        vec![geometry]
    }
}
