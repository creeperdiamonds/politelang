//! Words on the canvas.
//!
//! A picture that cannot say anything is only half a screen. This is a letter shape for every
//! character a program is likely to write, five dots across and seven down, kept as seven rows of
//! bits — the leftmost dot is the highest bit of the five.
//!
//! Drawn by hand, like the rest of it. A font file would have meant a parser for a font file, and
//! a dependency, and a thing to go missing; ninety-odd shapes fit here and go missing from nowhere.

use crate::canvas::{Canvas, Colour};

/// How wide and tall one letter is, before it is scaled up.
pub const WIDE: i64 = 5;
pub const TALL: i64 = 7;
/// The gap left between one letter and the next.
pub const GAP: i64 = 1;

/// The seven rows of one letter. An unknown character is drawn as a hollow box, so that something
/// missing looks missing rather than looking like a space.
fn rows_for(ch: char) -> [u8; 7] {
    match ch {
        ' ' => [0, 0, 0, 0, 0, 0, 0],

        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],

        'a' => [0b00000, 0b00000, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111],
        'b' => [0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110],
        'c' => [0b00000, 0b00000, 0b01111, 0b10000, 0b10000, 0b10000, 0b01111],
        'd' => [0b00001, 0b00001, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111],
        'e' => [0b00000, 0b00000, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110],
        'f' => [0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000],
        'g' => [0b00000, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        'h' => [0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b10001],
        'i' => [0b00100, 0b00000, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110],
        'j' => [0b00010, 0b00000, 0b00110, 0b00010, 0b00010, 0b10010, 0b01100],
        'k' => [0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010],
        'l' => [0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'm' => [0b00000, 0b00000, 0b11010, 0b10101, 0b10101, 0b10101, 0b10101],
        'n' => [0b00000, 0b00000, 0b11110, 0b10001, 0b10001, 0b10001, 0b10001],
        'o' => [0b00000, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110],
        'p' => [0b00000, 0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000],
        'q' => [0b00000, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001],
        'r' => [0b00000, 0b00000, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000],
        's' => [0b00000, 0b00000, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        't' => [0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110],
        'u' => [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101],
        'v' => [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'w' => [0b00000, 0b00000, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010],
        'x' => [0b00000, 0b00000, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001],
        'y' => [0b00000, 0b10001, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        'z' => [0b00000, 0b00000, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111],

        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],

        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        ',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b00100, 0b01000],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00100, 0b00000, 0b00100],
        '\'' => [0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000],
        '"' => [0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        ':' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
        ';' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b00100, 0b01000],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        ']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '\\' => [0b10000, 0b01000, 0b01000, 0b00100, 0b00010, 0b00010, 0b00001],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        '=' => [0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000],
        '*' => [0b00000, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0b00000],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        '#' => [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
        '%' => [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
        '&' => [0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101],
        '@' => [0b01110, 0b10001, 0b10111, 0b10101, 0b10111, 0b10000, 0b01110],

        _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111],
    }
}

/// How many dots across a line of writing takes up, at this size.
pub fn width_of(text: &str, size: i64) -> i64 {
    let size = size.clamp(1, 16);
    let letters = text.chars().count() as i64;
    if letters == 0 {
        return 0;
    }
    // Every letter, and a gap between each pair of them but not after the last.
    (letters * WIDE + (letters - 1) * GAP) * size
}

/// How many dots down a line of writing takes up, at this size.
pub fn height_of(size: i64) -> i64 {
    TALL * size.clamp(1, 16)
}

impl Canvas {
    /// Write a line of text with its top left corner here.
    ///
    /// Nothing is drawn for the dots a letter leaves blank, so writing over a picture leaves the
    /// picture showing between the strokes.
    pub fn write(&mut self, text: &str, x: i64, y: i64, colour: Colour, size: i64) {
        let size = size.clamp(1, 16);
        let mut at = x;
        for ch in text.chars() {
            let rows = rows_for(ch);
            for (down, row) in rows.iter().enumerate() {
                for across in 0..WIDE {
                    // The leftmost dot is the highest of the five bits.
                    if row & (1 << (WIDE - 1 - across)) == 0 {
                        continue;
                    }
                    if size == 1 {
                        self.paint(at + across, y + down as i64, colour);
                    } else {
                        self.fill_box(
                            at + across * size,
                            y + down as i64 * size,
                            at + across * size + size - 1,
                            y + down as i64 * size + size - 1,
                            colour,
                        );
                    }
                }
            }
            at += (WIDE + GAP) * size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_letter_puts_dots_where_its_shape_says() {
        let mut c = Canvas::new(20, 10);
        c.clear(0);
        c.write("L", 0, 0, 0xffffff, 1);
        // An L is a full left edge and a full bottom row, and nothing else.
        for down in 0..7 {
            assert_eq!(c.dot_at(0, down), 0xffffff, "the upright is missing at {down}");
        }
        for across in 0..5 {
            assert_eq!(c.dot_at(across, 6), 0xffffff, "the foot is missing at {across}");
        }
        // The top right corner of an L is blank.
        assert_eq!(c.dot_at(4, 0), 0);
    }

    #[test]
    fn a_space_marks_nothing_but_still_takes_room() {
        let mut c = Canvas::new(40, 10);
        c.clear(0);
        c.write("A A", 0, 0, 0xffffff, 1);
        // Between the two letters there is a gap, then the space, then a gap: all empty.
        for across in 5..12 {
            for down in 0..7 {
                assert_eq!(c.dot_at(across, down), 0, "something was drawn in the space");
            }
        }
        // And the second A did land after it.
        assert_eq!(c.dot_at(13, 0), 0xffffff);
    }

    #[test]
    fn the_width_agrees_with_what_was_drawn() {
        let mut c = Canvas::new(200, 20);
        c.clear(0);
        let words = "Hello there";
        c.write(words, 0, 0, 0xffffff, 2);
        let said = width_of(words, 2);
        // Nothing may be painted at or past the width it claims to need.
        for across in said..200 {
            for down in 0..20 {
                assert_eq!(c.dot_at(across, down), 0, "writing ran past its own width");
            }
        }
        assert!(said > 0);
    }

    #[test]
    fn size_makes_every_dot_a_square_of_that_side() {
        let mut c = Canvas::new(40, 40);
        c.clear(0);
        c.write("L", 0, 0, 0xffffff, 3);
        // The topmost dot of the upright becomes a three by three block.
        for across in 0..3 {
            for down in 0..3 {
                assert_eq!(c.dot_at(across, down), 0xffffff);
            }
        }
        assert_eq!(c.dot_at(3, 0), 0);
        assert_eq!(height_of(3), 21);
    }

    #[test]
    fn an_unknown_character_is_drawn_as_something_visibly_missing() {
        let mut c = Canvas::new(20, 10);
        c.clear(0);
        // A letter with no shape here becomes a hollow box, not a silent gap.
        c.write("\u{5d0}", 0, 0, 0xffffff, 1);
        assert_eq!(c.dot_at(0, 0), 0xffffff);
        assert_eq!(c.dot_at(4, 6), 0xffffff);
        assert_eq!(c.dot_at(2, 3), 0, "the box should be hollow");
    }

    #[test]
    fn writing_off_the_edge_is_quietly_ignored() {
        let mut c = Canvas::new(8, 8);
        c.clear(0);
        c.write("Something far too long to fit", -20, 200, 0xffffff, 4);
        for across in 0..8 {
            for down in 0..8 {
                assert_eq!(c.dot_at(across, down), 0);
            }
        }
    }
}
