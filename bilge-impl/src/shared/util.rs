use syn::Path;

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
        match str_segments.first().copied() {
            None => return false, // since path is non-empty, this is trivially false
            Some("std") => (), // then no need to touch first segment
            _ =>  str_segments.insert(0, "std"),
        };

        if self.matches(&str_segments) {
            return true
        }
        
        // try matching with "core" as first segment
        str_segments[0] = "core";
        self.matches(&str_segments)
    }
}

impl PathExt for Path {
    fn matches(&self, str_segments: &[&str]) -> bool {
        if self.segments.len() > str_segments.len() {
            return false
        }

        let segments = self.segments.iter().map(|seg| seg.ident.to_string()).rev();
        let str_segments = str_segments.iter().copied().rev();
        
        segments.zip(str_segments).all(|(a, b)| a == b)
    }
}