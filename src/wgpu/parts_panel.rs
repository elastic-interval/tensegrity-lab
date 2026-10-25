//! The parts panel down the left side in the connector view: a row per kind
//! of part (`connector::parts::kinds`), each with a rendering of that part
//! turning in a square and its count beside it. The rows sit at fixed screen
//! positions, so the panel stays put while the camera moves.

/// Gap from the window's edge, matching the text overlay's margin.
const MARGIN: f32 = 50.0;
/// Largest square a rendering gets, in pixels.
const MAX_THUMBNAIL: f32 = 110.0;
/// Space between a row's square and its text, and between rows.
const GAP: f32 = 16.0;

/// One row of the panel, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelRow {
    /// The rendering's square: left, top and side.
    pub thumbnail: [f32; 3],
    /// Where the row's text starts, centred on the square's height.
    pub text_at: [f32; 2],
}

/// Rows for `count` kinds in a window `width` × `height`, top down and
/// shrunk to fit when there are many.
pub fn panel_rows(width: f32, height: f32, count: usize) -> Vec<PanelRow> {
    if count == 0 {
        return Vec::new();
    }
    let pitch = ((height - 2.0 * MARGIN) / count as f32).min(MAX_THUMBNAIL + GAP);
    let side = (pitch - GAP).max(0.0).min(width / 4.0);
    let top = (height - pitch * count as f32) / 2.0;
    (0..count)
        .map(|row| {
            let y = top + pitch * row as f32;
            PanelRow {
                thumbnail: [MARGIN, y, side],
                text_at: [MARGIN + side + GAP, y + side / 2.0],
            }
        })
        .collect()
}
