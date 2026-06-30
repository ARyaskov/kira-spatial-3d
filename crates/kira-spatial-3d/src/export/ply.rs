use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::export::floatfmt::{FloatFmt, write_fmt_f32};
use crate::{Error, Mesh};

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

/// Write ASCII PLY to a stream.
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
        write_fmt_f32(&mut w, p[0], opts.float)?;
        w.write_all(b" ")?;
        write_fmt_f32(&mut w, p[1], opts.float)?;
        w.write_all(b" ")?;
        write_fmt_f32(&mut w, p[2], opts.float)?;
        if opts.write_normals {
            let n = mesh.normals[i];
            w.write_all(b" ")?;
            write_fmt_f32(&mut w, n[0], opts.float)?;
            w.write_all(b" ")?;
            write_fmt_f32(&mut w, n[1], opts.float)?;
            w.write_all(b" ")?;
            write_fmt_f32(&mut w, n[2], opts.float)?;
        }
        w.write_all(b"\n")?;
    }

    for tri in mesh.indices.chunks_exact(3) {
        writeln!(w, "3 {} {} {}", tri[0], tri[1], tri[2])?;
    }
    Ok(())
}

/// Save ASCII PLY to disk.
pub fn save_ply<P: AsRef<Path>>(mesh: &Mesh, path: P, opts: PlyOptions) -> Result<(), Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_ply(mesh, writer, opts)
}
