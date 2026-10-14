use super::*;

pub(super) struct Snapshot {
    pub bounds: Vec<Bounds>,
    pub paths: Vec<Vec<Point>>,
    pub junctions: Vec<Point>,
    scopes: Vec<Bounds>,
    bands: Vec<(usize, usize)>,
    pub width: usize,
    pub height: usize,
}

impl Snapshot {
    pub fn capture(layout: &Layout<'_>) -> Self {
        Self {
            bounds: layout.bounds.clone(),
            paths: layout.paths.clone(),
            junctions: layout.junctions.clone(),
            scopes: layout.scopes.iter().map(|s| s.bounds).collect(),
            bands: layout.bands.iter().map(|b| (b.top, b.height)).collect(),
            width: layout.width,
            height: layout.height,
        }
    }
    pub fn restore(self, layout: &mut Layout<'_>) {
        layout.bounds = self.bounds;
        layout.paths = self.paths;
        layout.junctions = self.junctions;
        for (scope, bounds) in layout.scopes.iter_mut().zip(self.scopes) {
            scope.bounds = bounds;
        }
        for (band, (top, height)) in layout.bands.iter_mut().zip(self.bands) {
            band.top = top;
            band.height = height;
        }
        layout.width = self.width;
        layout.height = self.height;
        layout.positions = layout.bounds.iter().map(|b| b.origin).collect();
    }
}
pub(super) fn allocate(
    layout: &mut Layout<'_>,
    horizontal: bool,
    cut: usize,
    amount: usize,
) -> bool {
    let axis = |p: Point| if horizontal { p.y } else { p.x };
    for (node, b) in layout.bounds.iter().enumerate() {
        let (low, high) = if horizontal {
            (b.origin.y, b.origin.y + b.height)
        } else {
            (b.origin.x, b.right())
        };
        if low <= cut
            && cut <= high
            && (!layout.containers.contains(&node)
                || (horizontal && low < cut && cut <= b.origin.y + layout.header_heights[node]))
        {
            return false;
        }
    }
    if horizontal
        && layout
            .scopes
            .iter()
            .any(|s| s.bounds.origin.y < cut && cut < s.bounds.origin.y + 36)
    {
        return false;
    }
    let move_point = |p: &mut Point| {
        if axis(*p) >= cut {
            if horizontal {
                p.y += amount;
            } else {
                p.x += amount;
            }
        }
    };
    let move_bounds = |b: &mut Bounds| {
        let (low, high) = if horizontal {
            (b.origin.y, b.origin.y + b.height)
        } else {
            (b.origin.x, b.right())
        };
        if low >= cut {
            move_point(&mut b.origin);
        } else if high >= cut {
            if horizontal {
                b.height += amount;
            } else {
                b.width += amount;
            }
        }
    };
    let mut paths = layout.paths.clone();
    translate_paths(&mut paths, horizontal, cut, amount);
    for b in &mut layout.bounds {
        move_bounds(b);
    }
    for s in &mut layout.scopes {
        move_bounds(&mut s.bounds);
    }
    layout.paths = paths;
    for p in &mut layout.junctions {
        move_point(p);
    }
    for band in &mut layout.bands {
        if horizontal {
            if band.top >= cut {
                band.top += amount;
            } else if band.top + band.height > cut {
                band.height += amount;
            }
        }
    }
    if horizontal {
        layout.height += amount;
    } else {
        layout.width += amount;
    }
    layout.positions = layout.bounds.iter().map(|b| b.origin).collect();
    true
}

pub(super) fn translate_paths(
    paths: &mut [Vec<Point>],
    horizontal: bool,
    cut: usize,
    amount: usize,
) {
    for point in paths.iter_mut().flatten() {
        let coordinate = if horizontal {
            &mut point.y
        } else {
            &mut point.x
        };
        if *coordinate >= cut {
            *coordinate += amount;
        }
    }
}
