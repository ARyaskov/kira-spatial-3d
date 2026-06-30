use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::export::floatfmt::{FloatFmt, write_fmt_f32};
use crate::{Error, Mesh};

#[derive(Clone, Copy, Debug)]
pub struct ObjOptions {
    pub float: FloatFmt,
    pub write_normals: bool,
}

impl Default for ObjOptions {
    fn default() -> Self {
        Self {
            float: FloatFmt::DEFAULT,
            write_normals: true,
        }
    }
}

/// Write ASCII OBJ to a stream.
pub fn write_obj<W: Write>(mesh: &Mesh, w: W, opts: ObjOptions) -> Result<(), Error> {
    let mut w = w;
    w.write_all(b"# kira-spatial-3d obj v1\n")?;

    for p in &mesh.vertices {
        w.write_all(b"v ")?;
        write_fmt_f32(&mut w, p[0], opts.float)?;
        w.write_all(b" ")?;
        write_fmt_f32(&mut w, p[1], opts.float)?;
        w.write_all(b" ")?;
        write_fmt_f32(&mut w, p[2], opts.float)?;
        w.write_all(b"\n")?;
    }

    if opts.write_normals {
        for n in &mesh.normals {
            w.write_all(b"vn ")?;
            write_fmt_f32(&mut w, n[0], opts.float)?;
            w.write_all(b" ")?;
            write_fmt_f32(&mut w, n[1], opts.float)?;
            w.write_all(b" ")?;
            write_fmt_f32(&mut w, n[2], opts.float)?;
            w.write_all(b"\n")?;
        }
    }

    for tri in mesh.indices.chunks_exact(3) {
        let a = tri[0] + 1;
        let b = tri[1] + 1;
        let c = tri[2] + 1;
        if opts.write_normals {
            writeln!(w, "f {a}//{a} {b}//{b} {c}//{c}")?;
        } else {
            writeln!(w, "f {a} {b} {c}")?;
        }
    }

    Ok(())
}

/// Save ASCII OBJ to disk.
pub fn save_obj<P: AsRef<Path>>(mesh: &Mesh, path: P, opts: ObjOptions) -> Result<(), Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_obj(mesh, writer, opts)
}
