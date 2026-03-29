use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::export::floatfmt::{FloatFmt, fmt_f32};
use crate::{Error, Mesh};

/// OBJ export settings.
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

/// Writes a deterministic ASCII OBJ stream.
pub fn write_obj<W: Write>(mesh: &Mesh, w: W, opts: ObjOptions) -> Result<(), Error> {
    let mut w = w;
    w.write_all(b"# kira-spatial-3d obj v1\n")?;

    for p in &mesh.vertices {
        writeln!(
            w,
            "v {} {} {}",
            fmt_f32(p[0], opts.float),
            fmt_f32(p[1], opts.float),
            fmt_f32(p[2], opts.float)
        )?;
    }

    if opts.write_normals {
        for n in &mesh.normals {
            writeln!(
                w,
                "vn {} {} {}",
                fmt_f32(n[0], opts.float),
                fmt_f32(n[1], opts.float),
                fmt_f32(n[2], opts.float)
            )?;
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

/// Saves deterministic ASCII OBJ to disk.
pub fn save_obj<P: AsRef<Path>>(mesh: &Mesh, path: P, opts: ObjOptions) -> Result<(), Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_obj(mesh, writer, opts)
}
