use std::{fmt::Display, path::Path};

/// Print a path that is broken into segments.
// explicitly typed to avoid type recursion. 'a is the smallest lifetime present
// : that of the child.
pub(crate) enum PathComponents<'a> {
    Path(&'a Path),
    Component(&'a PathComponents<'a>, &'a Path),
}

impl<'p> Display for PathComponents<'p> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathComponents::Path(p) => p.display().fmt(f),
            PathComponents::Component(p, c) => {
                p.fmt(f)?;
                f.write_str("/")?;
                c.display().fmt(f)
            }
        }
    }
}
