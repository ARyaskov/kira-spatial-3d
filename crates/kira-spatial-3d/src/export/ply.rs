use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::export::floatfmt::{FloatFmt, fmt_f32};
use crate::{Error, Mesh};

/// PLY export settings.
#[derive(Clone, Copy, Debug)]
pub struct PlyOptions {
    pub float: FloatFmt,
    pub write_normals: bool,
}

impl Default for PlyOptions {
    fn default() -> Self {
        Self {
            float: FloatFmt::DEFAULT,
            write_normals: true,
        }
    }
}

/// Writes deterministic ASCII PLY.
pub fn write_ply<W: Write>(mesh: &Mesh, w: W, opts: PlyOptions) -> Result<(), Error> {
    let mut w = w;
    let face_count = mesh.indices.len() / 3;

    writeln!(w, "ply")?;
    writeln!(w, "format ascii 1.0")?;
    writeln!(w, "comment kira-spatial-3d ply v1")?;
    writeln!(w, "element vertex {}", mesh.vertices.len())?;
    writeln!(w, "property float x")?;
    writeln!(w, "property float y")?;
    writeln!(w, "property float z")?;
    if opts.write_normals {
        writeln!(w, "property float nx")?;
        writeln!(w, "property float ny")?;
        writeln!(w, "property float nz")?;
    }
    writeln!(w, "element face {face_count}")?;
    writeln!(w, "property list uchar int vertex_indices")?;
    writeln!(w, "end_header")?;

    for (i, p) in mesh.vertices.iter().enumerate() {
        if opts.write_normals {
            let n = mesh.normals[i];
            writeln!(
                w,
                "{} {} {} {} {} {}",
                fmt_f32(p[0], opts.float),
                fmt_f32(p[1], opts.float),
                fmt_f32(p[2], opts.float),
                fmt_f32(n[0], opts.float),
                fmt_f32(n[1], opts.float),
                fmt_f32(n[2], opts.float)
            )?;
        } else {
            writeln!(
                w,
                "{} {} {}",
                fmt_f32(p[0], opts.float),
                fmt_f32(p[1], opts.float),
                fmt_f32(p[2], opts.float)
            )?;
        }
    }

    for tri in mesh.indices.chunks_exact(3) {
        writeln!(w, "3 {} {} {}", tri[0], tri[1], tri[2])?;
    }
    Ok(())
}

/// Saves deterministic ASCII PLY to disk.
pub fn save_ply<P: AsRef<Path>>(mesh: &Mesh, path: P, opts: PlyOptions) -> Result<(), Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_ply(mesh, writer, opts)
}
