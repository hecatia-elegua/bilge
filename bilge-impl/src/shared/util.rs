use syn::Path;
#[cfg(test)]
use syn_path::path;

pub trait PathExt {
    /// match path segments. `str_segments` should contain the entire
    /// qualified path from the crate root, for example `["bilge", "FromBits"]`.
    /// allows partial matches - `["std", "default", "Default"]` will also match
    /// the paths `Default` or `default::Default`.
    fn matches(&self, str_segments: &[&str]) -> bool;

    /// match path segments, but also allow first segment to be either "core" or "std"
    fn matches_core_or_std(&self, str_segments: &[&str]) -> bool {
        let mut str_segments = str_segments.to_owned();

        // try matching with "std" as first segment
        // first, make "std" the first segment
        match str_segments.first().copied() {
            None => return false, // since path is non-empty, this is trivially false
            Some("std") => (),
            _ => str_segments.insert(0, "std"),
        };

        if self.matches(&str_segments) {
            return true;
        }

        // try matching with "core" as first segment
        str_segments[0] = "core";
        self.matches(&str_segments)
    }
}

impl PathExt for Path {
    fn matches(&self, str_segments: &[&str]) -> bool {
        if self.segments.len() > str_segments.len() {
            return false;
        }

        let segments = self.segments.iter().map(|seg| seg.ident.to_string()).rev();
        let str_segments = str_segments.iter().copied().rev();

        segments.zip(str_segments).all(|(a, b)| a == b)
    }
}

#[test]
fn path_matching() {
    let paths = [
        path!(::std::default::Default),
        path!(std::default::Default),
        path!(default::Default),
        path!(Default),
    ];

    let str_segments = &["std", "default", "Default"];

    for path in paths {
        assert!(path.matches(str_segments));
    }
}

#[test]
fn partial_does_not_match() {
    let full_path = path!(std::foo::bar::fizz::Buzz);

    let str_segments = ["std", "foo", "bar", "fizz", "Buzz"];

    for i in 1..str_segments.len() {
        let partial_str_segments = &str_segments[i..];
        assert!(!full_path.matches(partial_str_segments))
    }
}

#[test]
fn path_matching_without_root() {
    let paths = [
        path!(::core::fmt::Debug),
        path!(core::fmt::Debug),
        path!(::std::fmt::Debug),
        path!(std::fmt::Debug),
        path!(fmt::Debug),
        path!(Debug),
    ];

    let str_segments_without_root = &["fmt", "Debug"];

    for path in paths {
        assert!(path.matches_core_or_std(str_segments_without_root));
    }
}
