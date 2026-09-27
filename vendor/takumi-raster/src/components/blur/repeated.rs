//! Collapse identical mask rows/columns while preserving all three box passes.
//! A three-pass blur has radius 3r per axis. Retaining 6r+1 equal samples keeps
//! one center whose entire dependency neighborhood lies inside the plateau.
//! Expand that *blurred* center, not an assumed original/opaque alpha value.

#[derive(Clone, Copy, Debug)]
struct Fold {
  center: usize,
  removed: usize,
}

impl Fold {
  fn new(start: usize, len: usize, radius: usize) -> Self {
    let support = radius.saturating_mul(3);
    let keep = support.saturating_mul(2).saturating_add(1);
    if len > keep {
      Self {
        center: start + support,
        removed: len - keep,
      }
    } else {
      Self {
        center: 0,
        removed: 0,
      }
    }
  }

  fn source(self, compact: usize) -> usize {
    compact
      + if compact > self.center {
        self.removed
      } else {
        0
      }
  }

  fn compact(self, original: usize) -> usize {
    if original <= self.center {
      original
    } else if original <= self.center + self.removed {
      self.center
    } else {
      original - self.removed
    }
  }
}

pub(super) struct RepeatedMask {
  pub pixels: Vec<u8>,
  pub width: u32,
  pub height: u32,
  x: Fold,
  y: Fold,
}

impl RepeatedMask {
  pub fn new(data: &[u8], width: u32, height: u32, radius: u32) -> Option<Self> {
    let w = width as usize;
    let h = height as usize;
    // Tiny/irregular masks are cheaper on the general path.
    if data.len() < 4096 || w == 0 || h == 0 {
      return None;
    }
    let mut differences = vec![0u8; w - 1];
    let mut previous: Option<&[u8]> = None;
    let (mut row_start, mut row_len) = (0, 0);
    let (mut best_start, mut best_len) = (0, 0);
    for (y, row) in data.chunks_exact(w).enumerate() {
      if previous == Some(row) {
        row_len += 1;
      } else {
        row_start = y;
        row_len = 1;
        // A zero difference means these adjacent columns agree on EVERY row.
        // Identical rows need not contribute again. The contiguous loop also
        // permits auto-vectorization without architecture-specific intrinsics.
        for x in 0..w - 1 {
          differences[x] |= row[x] ^ row[x + 1];
        }
      }
      if row_len > best_len {
        (best_start, best_len) = (row_start, row_len);
      }
      previous = Some(row);
    }
    let y = Fold::new(best_start, best_len, radius as usize);
    let (mut col_start, mut col_len) = (0, 1);
    let (mut best_start, mut best_len) = (0, 1);
    for (column, difference) in differences.into_iter().enumerate() {
      if difference == 0 {
        col_len += 1;
      } else {
        col_start = column + 1;
        col_len = 1;
      }
      if col_len > best_len {
        (best_start, best_len) = (col_start, col_len);
      }
    }
    let x = Fold::new(best_start, best_len, radius as usize);
    let cw = w - x.removed;
    let ch = h - y.removed;
    if cw * ch > data.len() - data.len() / 4 {
      return None;
    }
    let mut pixels = vec![0; cw * ch];
    for (cy, row) in pixels.chunks_exact_mut(cw).enumerate() {
      let original = &data[y.source(cy) * w..][..w];
      let split = x.center + 1;
      row[..split].copy_from_slice(&original[..split]);
      row[split..].copy_from_slice(&original[split + x.removed..]);
    }
    Some(Self {
      pixels,
      width: cw as u32,
      height: ch as u32,
      x,
      y,
    })
  }

  pub fn expand_into(&self, data: &mut [u8], width: u32) {
    let cw = self.width as usize;
    for (y, row) in data.chunks_exact_mut(width as usize).enumerate() {
      let compact = &self.pixels[self.y.compact(y) * cw..][..cw];
      let split = self.x.center + 1;
      row[..split].copy_from_slice(&compact[..split]);
      row[split..split + self.x.removed].fill(compact[self.x.center]);
      row[split + self.x.removed..].copy_from_slice(&compact[split..]);
    }
  }
}
