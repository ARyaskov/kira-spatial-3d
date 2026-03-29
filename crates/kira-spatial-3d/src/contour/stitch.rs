use std::collections::{BTreeMap, BTreeSet};

use crate::Error;
use crate::contour::types::ContourSet;

/// Quantization settings for deterministic contour endpoint snapping.
#[derive(Clone, Copy, Debug)]
pub struct Quantize {
    pub grid: f32,
}

/// Quantized 2D key for contour topology reconstruction.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct QKey {
    pub qx: i32,
    pub qy: i32,
}

/// A stitched contour polyline for one iso-level.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    pub level: f32,
    pub points: Vec<[f32; 3]>,
    pub is_closed: bool,
}

/// Collection of stitched polylines for one iso-level.
#[derive(Debug, Clone, PartialEq)]
pub struct PolylineSet {
    pub level: f32,
    pub polylines: Vec<Polyline>,
}

/// Stitching options.
#[derive(Clone, Copy, Debug)]
pub struct StitchOptions {
    pub quantize: Quantize,
}

/// Builds a quantized key for a 3D point.
///
/// Quantization uses `x` and `y` only; `z` is ignored.
#[inline]
pub fn qkey(p: [f32; 3], q: Quantize) -> QKey {
    let qx = (p[0] / q.grid).round() as i32;
    let qy = (p[1] / q.grid).round() as i32;
    QKey { qx, qy }
}

/// Deterministically stitches unordered contour segments into polylines.
///
/// # Determinism
/// - Endpoint snapping is stable via fixed-grid quantization.
/// - Canonical point representatives are first-seen in segment order.
/// - Path walking is deterministic via ordered maps/sets and smallest-neighbor choice.
/// - Closed loops are canonically rotated/oriented by lexicographic `QKey`.
pub fn stitch_contours(set: &ContourSet, opts: StitchOptions) -> Result<PolylineSet, Error> {
    validate_quantize(opts.quantize)?;

    let mut adjacency: BTreeMap<QKey, Vec<QKey>> = BTreeMap::new();
    let mut canonical: BTreeMap<QKey, [f32; 3]> = BTreeMap::new();
    let mut unused: BTreeSet<(QKey, QKey)> = BTreeSet::new();

    for segment in &set.segments {
        let k0 = qkey(segment.p0, opts.quantize);
        let k1 = qkey(segment.p1, opts.quantize);
        if k0 == k1 {
            continue;
        }

        canonical
            .entry(k0)
            .or_insert([segment.p0[0], segment.p0[1], set.level]);
        canonical
            .entry(k1)
            .or_insert([segment.p1[0], segment.p1[1], set.level]);

        adjacency.entry(k0).or_default().push(k1);
        adjacency.entry(k1).or_default().push(k0);

        unused.insert((k0, k1));
        unused.insert((k1, k0));
    }

    let mut keyed = Vec::<(bool, Vec<QKey>)>::new();

    for (&node, neighbors) in &adjacency {
        if neighbors.len() != 1 {
            continue;
        }
        if !has_available_neighbor(node, &adjacency, &unused) {
            continue;
        }
        let mut keys = walk_path(node, &adjacency, &mut unused);
        if keys.len() < 2 {
            continue;
        }
        canonicalize_open(&mut keys);
        keyed.push((false, keys));
    }

    while let Some(&(start, _)) = unused.iter().next() {
        let mut keys = walk_loop(start, &adjacency, &mut unused);
        if keys.len() < 3 {
            continue;
        }
        canonicalize_closed(&mut keys);
        keyed.push((true, keys));
    }

    keyed.sort_by(|a, b| (a.0, a.1[0], a.1.len()).cmp(&(b.0, b.1[0], b.1.len())));

    let mut polylines = Vec::with_capacity(keyed.len());
    for (is_closed, keys) in keyed {
        let points = keys
            .iter()
            .map(|k| {
                let mut p = canonical[k];
                p[2] = set.level;
                p
            })
            .collect::<Vec<_>>();
        polylines.push(Polyline {
            level: set.level,
            points,
            is_closed,
        });
    }

    Ok(PolylineSet {
        level: set.level,
        polylines,
    })
}

fn validate_quantize(q: Quantize) -> Result<(), Error> {
    if !q.grid.is_finite() || q.grid <= 0.0 {
        return Err(Error::InvalidContourSpec {
            message: "quantize.grid must be finite and > 0",
        });
    }
    Ok(())
}

fn has_available_neighbor(
    node: QKey,
    adjacency: &BTreeMap<QKey, Vec<QKey>>,
    unused: &BTreeSet<(QKey, QKey)>,
) -> bool {
    adjacency
        .get(&node)
        .is_some_and(|ns| ns.iter().any(|&n| unused.contains(&(node, n))))
}

fn walk_path(
    start: QKey,
    adjacency: &BTreeMap<QKey, Vec<QKey>>,
    unused: &mut BTreeSet<(QKey, QKey)>,
) -> Vec<QKey> {
    let mut keys = vec![start];
    let mut cur = start;

    loop {
        let next = next_neighbor(cur, adjacency, unused);
        let Some(n) = next else {
            break;
        };
        remove_edge_pair(cur, n, unused);
        cur = n;
        keys.push(cur);
    }

    keys
}

fn walk_loop(
    start: QKey,
    adjacency: &BTreeMap<QKey, Vec<QKey>>,
    unused: &mut BTreeSet<(QKey, QKey)>,
) -> Vec<QKey> {
    let mut keys = vec![start];
    let mut cur = start;

    loop {
        let next = next_neighbor(cur, adjacency, unused);
        let Some(n) = next else {
            break;
        };
        remove_edge_pair(cur, n, unused);
        cur = n;
        keys.push(cur);
        if cur == start {
            break;
        }
    }

    if keys.first() == keys.last() {
        let _ = keys.pop();
    }
    keys
}

#[inline]
fn next_neighbor(
    cur: QKey,
    adjacency: &BTreeMap<QKey, Vec<QKey>>,
    unused: &BTreeSet<(QKey, QKey)>,
) -> Option<QKey> {
    adjacency.get(&cur).and_then(|neighbors| {
        neighbors
            .iter()
            .copied()
            .filter(|&n| unused.contains(&(cur, n)))
            .min()
    })
}

#[inline]
fn remove_edge_pair(a: QKey, b: QKey, unused: &mut BTreeSet<(QKey, QKey)>) {
    let _ = unused.remove(&(a, b));
    let _ = unused.remove(&(b, a));
}

fn canonicalize_open(keys: &mut [QKey]) {
    if keys.is_empty() {
        return;
    }
    let start = keys[0];
    let end = keys[keys.len() - 1];
    if end < start {
        keys.reverse();
    }
}

fn canonicalize_closed(keys: &mut Vec<QKey>) {
    if keys.len() < 2 {
        return;
    }

    let (min_idx, _) = keys
        .iter()
        .enumerate()
        .min_by_key(|(_, k)| **k)
        .expect("non-empty loop");
    keys.rotate_left(min_idx);

    let mut rev = Vec::with_capacity(keys.len());
    rev.push(keys[0]);
    rev.extend(keys.iter().skip(1).rev().copied());

    let choose_rev = keys
        .get(1)
        .zip(rev.get(1))
        .is_some_and(|(fwd_next, rev_next)| rev_next < fwd_next);
    if choose_rev {
        *keys = rev;
    }
}
