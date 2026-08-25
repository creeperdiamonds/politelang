//! Drawing.
//!
//! A terminal is a grid of cells, and the upper-half-block character lets one cell hold two
//! pixels: the top one painted as the letter's colour, the bottom as the colour behind it. With
//! 24-bit colour that is a real framebuffer, and it needs nothing but text on the way out — no
//! window, no library, no unsafe code (spec 9.5).
//!
//! Coordinates count from the top left corner, across first and then down, the way a page is read.

/// A colour, kept as one whole number: red, green and blue packed in that order.
pub type Colour = u32;

pub fn colour_of(red: i64, green: i64, blue: i64) -> Colour {
    let clamp = |v: i64| v.clamp(0, 255) as u32;
    (clamp(red) << 16) | (clamp(green) << 8) | clamp(blue)
}

pub fn parts_of(c: Colour) -> (u8, u8, u8) {
    (((c >> 16) & 255) as u8, ((c >> 8) & 255) as u8, (c & 255) as u8)
}

/// The colours everybody knows by name, so a program does not have to spell out three numbers to
/// draw something red.
pub fn colour_called(name: &str) -> Option<Colour> {
    Some(match name.trim().to_ascii_lowercase().as_str() {
        "black" => 0x000000,
        "white" => 0xffffff,
        "grey" | "gray" => 0x808080,
        "light grey" | "light gray" => 0xc0c0c0,
        "dark grey" | "dark gray" => 0x404040,
        "red" => 0xd23b3b,
        "dark red" => 0x7a1f1f,
        "green" => 0x3faa63,
        "dark green" => 0x1f5c34,
        "blue" => 0x3d7ad2,
        "dark blue" => 0x1b3a6b,
        "yellow" => 0xe8c53d,
        "orange" => 0xe08a2e,
        "brown" => 0x7a5230,
        "purple" => 0x8a4fbf,
        "pink" => 0xe08ab0,
        "sky" => 0x6fa8dc,
        "sand" => 0xd9c398,
        "stone" => 0x6b7280,
        "moss" => 0x5f7a4a,
        "bone" => 0xe8e3d3,
        "rust" => 0xa4552b,
        "lamp" => 0xe8a33d,
        "night" => 0x10151d,
        _ => return None,
    })
}

pub struct Canvas {
    pub wide: usize,
    pub tall: usize,
    dots: Vec<Colour>,
}

impl Canvas {
    /// A canvas is capped at a size a terminal could plausibly show, so a slip of the finger
    /// cannot ask for a gigabyte of picture.
    pub fn new(wide: i64, tall: i64) -> Canvas {
        let wide = wide.clamp(1, 400) as usize;
        let tall = tall.clamp(1, 400) as usize;
        Canvas {
            wide,
            tall,
            dots: vec![0; wide * tall],
        }
    }

    pub fn clear(&mut self, colour: Colour) {
        for d in self.dots.iter_mut() {
            *d = colour;
        }
    }

    /// Painting outside the canvas is quietly ignored rather than refused: a picture that runs off
    /// the edge is a normal thing to draw, not a mistake to be told off for.
    pub fn paint(&mut self, x: i64, y: i64, colour: Colour) {
        if x < 0 || y < 0 || x >= self.wide as i64 || y >= self.tall as i64 {
            return;
        }
        let at = y as usize * self.wide + x as usize;
        self.dots[at] = colour;
    }

    pub fn dot_at(&self, x: i64, y: i64) -> Colour {
        if x < 0 || y < 0 || x >= self.wide as i64 || y >= self.tall as i64 {
            return 0;
        }
        self.dots[y as usize * self.wide + x as usize]
    }

    /// A straight line, walked one dot at a time by Bresenham's method: only whole numbers, and
    /// never a gap.
    pub fn line(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, colour: Colour) {
        let (mut x, mut y) = (x0, y0);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut guard = 0;
        loop {
            self.paint(x, y, colour);
            if x == x1 && y == y1 {
                break;
            }
            guard += 1;
            if guard > 4000 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn fill_box(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, colour: Colour) {
        let (left, right) = (x0.min(x1), x0.max(x1));
        let (top, bottom) = (y0.min(y1), y0.max(y1));
        for y in top..=bottom {
            for x in left..=right {
                self.paint(x, y, colour);
            }
        }
    }

    pub fn outline_box(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, colour: Colour) {
        let (left, right) = (x0.min(x1), x0.max(x1));
        let (top, bottom) = (y0.min(y1), y0.max(y1));
        self.line(left, top, right, top, colour);
        self.line(left, bottom, right, bottom, colour);
        self.line(left, top, left, bottom, colour);
        self.line(right, top, right, bottom, colour);
    }

    /// A filled circle, by asking of every dot in the square around it whether it is near enough.
    pub fn circle(&mut self, cx: i64, cy: i64, radius: i64, colour: Colour) {
        let r = radius.max(0);
        let rr = r * r;
        for y in (cy - r)..=(cy + r) {
            for x in (cx - r)..=(cx + r) {
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy <= rr {
                    self.paint(x, y, colour);
                }
            }
        }
    }

    /// The picture, ready to be printed.
    ///
    /// Two rows of dots share one row of the terminal: the top one is painted as the letter and
    /// the bottom as the ground behind it.
    pub fn to_blocks(&self) -> String {
        let mut out = String::with_capacity(self.wide * self.tall * 12);
        // Home the cursor rather than clearing, so the picture does not flicker between frames.
        out.push_str("\u{1b}[H");
        let mut y = 0;
        while y < self.tall {
            let mut last_top: Option<Colour> = None;
            let mut last_bottom: Option<Colour> = None;
            for x in 0..self.wide {
                let top = self.dots[y * self.wide + x];
                let bottom = if y + 1 < self.tall {
                    self.dots[(y + 1) * self.wide + x]
                } else {
                    0
                };
                // Only say a colour when it changes, which cuts the output by a good half.
                if last_top != Some(top) {
                    let (r, g, b) = parts_of(top);
                    out.push_str(&format!("\u{1b}[38;2;{r};{g};{b}m"));
                    last_top = Some(top);
                }
                if last_bottom != Some(bottom) {
                    let (r, g, b) = parts_of(bottom);
                    out.push_str(&format!("\u{1b}[48;2;{r};{g};{b}m"));
                    last_bottom = Some(bottom);
                }
                out.push('\u{2580}'); // the upper half block
            }
            out.push_str("\u{1b}[0m\n");
            y += 2;
        }
        out
    }

    /// The same picture in plain letters, for anywhere that colour does not come out.
    ///
    /// Nothing is hidden from anybody: a terminal that cannot do colour still gets the shape.
    pub fn to_letters(&self) -> String {
        const SHADES: &[u8] = b" .:-=+*#%@";
        let mut out = String::with_capacity(self.wide * self.tall / 2 + self.tall);
        let mut y = 0;
        while y < self.tall {
            for x in 0..self.wide {
                let top = self.dots[y * self.wide + x];
                let bottom = if y + 1 < self.tall {
                    self.dots[(y + 1) * self.wide + x]
                } else {
                    top
                };
                let brightness = |c: Colour| {
                    let (r, g, b) = parts_of(c);
                    // The eye is far more taken by green than by blue.
                    (r as u32 * 30 + g as u32 * 59 + b as u32 * 11) / 100
                };
                let level = (brightness(top) + brightness(bottom)) / 2;
                let step = (level as usize * (SHADES.len() - 1)) / 255;
                out.push(SHADES[step] as char);
            }
            out.push('\n');
            y += 2;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_packs_and_unpacks() {
        let c = colour_of(255, 128, 0);
        assert_eq!(parts_of(c), (255, 128, 0));
        // Anything outside nought to two hundred and fifty five is brought back inside.
        assert_eq!(parts_of(colour_of(-40, 999, 12)), (0, 255, 12));
    }

    #[test]
    fn colours_have_names_worth_knowing() {
        assert_eq!(colour_called("red"), Some(0xd23b3b));
        assert_eq!(colour_called("  WHITE "), Some(0xffffff));
        assert_eq!(colour_called("chartreuse"), None);
    }

    #[test]
    fn painting_off_the_edge_is_quietly_ignored() {
        let mut c = Canvas::new(10, 10);
        c.paint(-5, 3, 0xffffff);
        c.paint(50, 3, 0xffffff);
        c.paint(3, -1, 0xffffff);
        c.paint(3, 99, 0xffffff);
        assert_eq!(c.dot_at(3, 3), 0);
    }

    #[test]
    fn a_line_reaches_both_of_its_ends_with_no_gaps() {
        let mut c = Canvas::new(40, 40);
        c.line(2, 2, 30, 20, 0xffffff);
        assert_ne!(c.dot_at(2, 2), 0, "the start should be painted");
        assert_ne!(c.dot_at(30, 20), 0, "the end should be painted");
        // Every row it passes through has at least one dot in it.
        for y in 2..=20 {
            let mut any = false;
            for x in 0..40 {
                if c.dot_at(x, y) != 0 {
                    any = true;
                }
            }
            assert!(any, "row {y} was skipped");
        }
    }

    #[test]
    fn a_box_fills_inside_and_not_outside() {
        let mut c = Canvas::new(20, 20);
        c.fill_box(5, 5, 9, 9, 0xff0000);
        assert_eq!(c.dot_at(5, 5), 0xff0000);
        assert_eq!(c.dot_at(9, 9), 0xff0000);
        assert_eq!(c.dot_at(4, 5), 0);
        assert_eq!(c.dot_at(10, 9), 0);
        // Given back to front, it comes out the same.
        let mut d = Canvas::new(20, 20);
        d.fill_box(9, 9, 5, 5, 0xff0000);
        assert_eq!(d.dot_at(7, 7), 0xff0000);
    }

    #[test]
    fn a_circle_is_round() {
        let mut c = Canvas::new(40, 40);
        c.circle(20, 20, 8, 0x00ff00);
        assert_eq!(c.dot_at(20, 20), 0x00ff00);
        assert_eq!(c.dot_at(28, 20), 0x00ff00);
        assert_eq!(c.dot_at(20, 28), 0x00ff00);
        // The corner of the square around it is outside the circle.
        assert_eq!(c.dot_at(27, 27), 0);
    }

    #[test]
    fn two_rows_of_dots_become_one_row_of_the_terminal() {
        let mut c = Canvas::new(4, 4);
        c.clear(0x102030);
        let blocks = c.to_blocks();
        assert_eq!(blocks.matches('\u{2580}').count(), 8, "four across, two rows deep");
        assert!(blocks.contains("38;2;16;32;48"), "should name the colour once");
        let letters = c.to_letters();
        assert_eq!(letters.lines().count(), 2);
        assert_eq!(letters.lines().next().unwrap().chars().count(), 4);
    }

    #[test]
    fn the_letters_get_lighter_as_the_picture_does() {
        let mut dark = Canvas::new(2, 2);
        dark.clear(0x000000);
        let mut bright = Canvas::new(2, 2);
        bright.clear(0xffffff);
        assert!(dark.to_letters().starts_with(' '));
        assert!(bright.to_letters().starts_with('@'));
    }

    #[test]
    fn a_canvas_cannot_be_asked_for_more_than_a_screen() {
        let c = Canvas::new(100_000, 100_000);
        assert_eq!(c.wide, 400);
        assert_eq!(c.tall, 400);
    }
}
