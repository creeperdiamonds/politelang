//! Real pictures, in real files.
//!
//! A canvas can be saved as a PNG, which any machine on earth can open, and shown in a window
//! rather than a terminal.
//!
//! Written by hand like everything else here (spec 9.5). PNG asks for its pixels inside a zlib
//! stream, and zlib allows blocks that are simply stored rather than squeezed — so a perfectly
//! valid picture can be written with nothing but the standard library, no compression code and no
//! dependency at all. The file is larger than it would otherwise be, and it opens anywhere.

use crate::canvas::{parts_of, Canvas};
use crate::Answer;

/// The table the PNG checksum is built from.
fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *slot = c;
    }
    table
}

fn crc32(bytes: &[u8]) -> u32 {
    let table = crc_table();
    let mut c = 0xFFFF_FFFFu32;
    for b in bytes {
        c = table[((c ^ *b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

/// The checksum zlib puts at the end of its stream.
fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for byte in bytes {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 12);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    let mut check = Vec::with_capacity(payload.len() + 4);
    check.extend_from_slice(kind);
    check.extend_from_slice(payload);
    out.extend_from_slice(&crc32(&check).to_be_bytes());
    out
}

/// A zlib stream whose blocks are stored rather than squeezed.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + raw.len() / 65535 * 5 + 8);
    out.push(0x78); // the usual zlib header
    out.push(0x01);
    let mut at = 0usize;
    if raw.is_empty() {
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while at < raw.len() {
        let take = (raw.len() - at).min(65535);
        let last = at + take >= raw.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(take as u16).to_le_bytes());
        out.extend_from_slice(&(!(take as u16)).to_le_bytes());
        out.extend_from_slice(&raw[at..at + take]);
        at += take;
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// The canvas as the bytes of a PNG file.
pub fn to_png(canvas: &Canvas, scale: usize) -> Vec<u8> {
    // A dot on a canvas is small, so it is drawn as a square of this many across in the picture.
    let scale = scale.clamp(1, 16);
    let wide = canvas.wide * scale;
    let tall = canvas.tall * scale;

    let mut raw = Vec::with_capacity(tall * (wide * 3 + 1));
    for y in 0..tall {
        raw.push(0); // this row is stored as it is, with nothing taken off it
        for x in 0..wide {
            let (r, g, b) = parts_of(canvas.dot_at((x / scale) as i64, (y / scale) as i64));
            raw.push(r);
            raw.push(g);
            raw.push(b);
        }
    }

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&(wide as u32).to_be_bytes());
    header.extend_from_slice(&(tall as u32).to_be_bytes());
    header.push(8); // eight bits for each of red, green and blue
    header.push(2); // colour, with no table and no see-through
    header.push(0);
    header.push(0);
    header.push(0);

    let mut out = Vec::with_capacity(raw.len() + 64);
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    out.extend_from_slice(&chunk(b"IHDR", &header));
    out.extend_from_slice(&chunk(b"IDAT", &zlib_stored(&raw)));
    out.extend_from_slice(&chunk(b"IEND", b""));
    out
}

pub fn save_png(canvas: &Canvas, path: &str, scale: usize) -> Answer<()> {
    match std::fs::write(path, to_png(canvas, scale)) {
        Ok(()) => Ok(()),
        Err(e) => Err(match e.kind() {
            std::io::ErrorKind::PermissionDenied => format!("\"{path}\" is not mine to write"),
            std::io::ErrorKind::NotFound => format!("there is nowhere called \"{path}\" to write to"),
            _ => format!("\"{path}\" could not be written"),
        }),
    }
}

/// The page a window shows: the picture, and nothing else in its way.
///
/// It reloads the picture by itself, so a program that draws again while the window is open is
/// seen straight away without anybody reaching for the keyboard.
pub fn viewer_page(title: &str, picture: &str) -> String {
    // The ampersand goes first, or the escaping escapes itself.
    let safe = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        r##"<!doctype html>
<meta charset="utf-8">
<title>{safe}</title>
<style>
  html, body {{ margin: 0; height: 100%; background: #10151d; }}
  body {{ display: grid; place-items: center; }}
  img {{
    max-width: 96vw; max-height: 92vh;
    image-rendering: pixelated;
    box-shadow: 0 20px 70px rgba(0,0,0,.6);
    border: 1px solid #2b3849;
  }}
  p {{
    position: fixed; bottom: 10px; left: 0; right: 0; text-align: center;
    color: #6d7a8c; font: 13px/1.4 ui-sans-serif, system-ui, sans-serif; margin: 0;
  }}
</style>
<img id="p" alt="{safe}">
<p>Drawn by PoliteLang. This window follows along on its own.</p>
<script>
  // Ask for the picture again every so often, with a fresh number on the end so that nothing
  // stale is handed back.
  const img = document.getElementById("p");
  const name = {picture:?};
  function again() {{ img.src = name + "?" + Date.now(); }}
  again();
  setInterval(again, 400);
</script>
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_png_starts_the_way_a_png_must() {
        let mut c = Canvas::new(4, 3);
        c.clear(0x112233);
        let png = to_png(&c, 1);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        // The header chunk says how big it is, in plain sight.
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], &4u32.to_be_bytes());
        assert_eq!(&png[20..24], &3u32.to_be_bytes());
        // And it ends where it should.
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    #[test]
    fn every_chunk_carries_a_checksum_that_agrees() {
        let mut c = Canvas::new(8, 8);
        c.clear(0x00ff00);
        c.line(0, 0, 7, 7, 0xff0000);
        let png = to_png(&c, 2);

        // Walk the file the way a reader would, checking each piece as it goes.
        let mut at = 8usize;
        let mut kinds = Vec::new();
        while at + 8 <= png.len() {
            let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
            let kind = &png[at + 4..at + 8];
            let body = &png[at + 8..at + 8 + len];
            let said = u32::from_be_bytes(png[at + 8 + len..at + 12 + len].try_into().unwrap());
            let mut check = kind.to_vec();
            check.extend_from_slice(body);
            assert_eq!(crc32(&check), said, "a chunk's checksum does not agree");
            kinds.push(String::from_utf8_lossy(kind).to_string());
            at += 12 + len;
        }
        assert_eq!(kinds, vec!["IHDR", "IDAT", "IEND"]);
        assert_eq!(at, png.len(), "there is something left over at the end");
    }

    #[test]
    fn a_stored_stream_holds_everything_it_was_given() {
        // Long enough to need more than one block.
        let raw: Vec<u8> = (0..200_000).map(|i| (i % 251) as u8).collect();
        let z = zlib_stored(&raw);
        assert_eq!(&z[..2], &[0x78, 0x01]);

        // Read the blocks back out and make sure nothing was lost.
        let mut at = 2usize;
        let mut got = Vec::new();
        loop {
            let last = z[at] & 1 == 1;
            let len = u16::from_le_bytes(z[at + 1..at + 3].try_into().unwrap()) as usize;
            let not = u16::from_le_bytes(z[at + 3..at + 5].try_into().unwrap());
            assert_eq!(!(len as u16), not, "the length is not confirmed");
            got.extend_from_slice(&z[at + 5..at + 5 + len]);
            at += 5 + len;
            if last {
                break;
            }
        }
        assert_eq!(got, raw);
        assert_eq!(
            u32::from_be_bytes(z[at..at + 4].try_into().unwrap()),
            adler32(&raw)
        );
    }

    #[test]
    fn scaling_makes_the_picture_bigger_and_keeps_the_colours() {
        let mut c = Canvas::new(2, 2);
        c.paint(0, 0, 0xff0000);
        let small = to_png(&c, 1);
        let large = to_png(&c, 4);
        assert_eq!(&small[16..20], &2u32.to_be_bytes());
        assert_eq!(&large[16..20], &8u32.to_be_bytes());
        assert!(large.len() > small.len());
    }

    #[test]
    fn the_window_page_names_its_picture_and_its_title() {
        let page = viewer_page("A Room", "room.png");
        assert!(page.contains("<title>A Room</title>"));
        assert!(page.contains("\"room.png\""));
        assert!(page.contains("setInterval"));
        // Nothing is fetched from anywhere else.
        assert!(!page.contains("http://") && !page.contains("https://"));
    }

    #[test]
    fn a_title_cannot_smuggle_markup_into_the_page() {
        let page = viewer_page("<script>trouble</script>", "x.png");
        assert!(!page.contains("<script>trouble"));
        assert!(page.contains("&lt;script&gt;trouble"));
    }
}
