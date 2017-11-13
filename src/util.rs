use std::{env, str};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};


/// Calls `File::open(path)` and if the result is `Err`, replace the error with new one which
/// message contains `path`.
pub fn open_file(path: &Path) -> io::Result<File> {
    File::open(path).map_err(|e| {
        let message = match e.kind() {
            io::ErrorKind::NotFound => format!("No such file: {:?}", path),
            _ => format!("An IO error occured while opening {:?}: {}", path, e),
        };
        io::Error::new(e.kind(), message)
    })
}


/// Calls `fs::create_dir_all` and `File::create`.
pub fn create_file_and_dirs(path: &Path) -> io::Result<File> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    File::create(path).map_err(|e| {
        let message = format!(
            "An IO error occured while opening/creating {:?}: {}",
            path,
            e
        );
        io::Error::new(e.kind(), message)
    })
}


/// Returns a `String` read from `read`.
pub fn string_from_read<R: Read>(read: R) -> io::Result<String> {
    let mut read = read;
    let mut buf = String::new();
    read.read_to_string(&mut buf)?;
    Ok(buf)
}


/// Equals to `string_from_read(open_file(path)?)`.
pub fn string_from_file_path(path: &Path) -> io::Result<String> {
    string_from_read(open_file(path)?)
}


/// Prints the given string ignoring the last newline if it exists.
pub fn eprintln_trimming_last_newline(s: &str) {
    if s.chars().last() == Some('\n') {
        eprint_and_flush!("{}", s);
    } else {
        eprintln!("{}", s);
    }
}


/// Returns `~/<names>` as `io::Result`.
///
/// # Errors
///
/// Returns `Err` if a home directory not found.
pub fn path_under_home(names: &[&str]) -> io::Result<PathBuf> {
    let home_dir = env::home_dir().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "Home directory not found",
    ))?;
    Ok(names.iter().fold(home_dir, |mut path, name| {
        path.push(name);
        path
    }))
}


pub trait OkAsRefOr {
    type Item;
    /// Get the value `Ok(&x)` if `Some(ref x) = self`.
    ///
    /// # Errors
    ///
    /// Returns `Err(e)` if `self` is `None`.
    fn ok_as_ref_or<E>(&self, e: E) -> Result<&Self::Item, E>;
}

impl<T> OkAsRefOr for Option<T> {
    type Item = T;

    fn ok_as_ref_or<E>(&self, e: E) -> Result<&T, E> {
        match *self {
            Some(ref x) => Ok(x),
            None => Err(e),
        }
    }
}


pub trait ToCamlCase {
    /// Converts `self` to CamlCase.
    fn to_caml_case(&self) -> String;
}

impl ToCamlCase for str {
    fn to_caml_case(&self) -> String {
        let mut s = String::new();
        let mut p = true;
        self.chars().foreach(|c| match c.to_uppercase().next() {
            Some('-') | Some('_') => {
                p = true;
            }
            Some(c) if p => {
                p = false;
                s.push(c)
            }
            _ => s.push(c),
        });
        s
    }
}


pub trait Foreach {
    type Item;
    /// A substitution of `Iterator::for_each`, which is unstable until Rust 1.21.
    fn foreach<F: FnMut(Self::Item)>(self, f: F);
}

impl<T, I: Iterator<Item = T>> Foreach for I {
    type Item = T;

    fn foreach<F: FnMut(T)>(self, mut f: F) {
        self.fold((), move |(), x| f(x));
    }
}
