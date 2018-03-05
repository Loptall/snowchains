pub mod atcoder;
pub mod atcoder_beta;
pub mod hackerrank;

use errors::{ServiceError, ServiceErrorKind, ServiceResult};
use terminal::Color;
use util;

use {rpassword, rprompt, webbrowser};
use httpsession::{self, ColorMode, CookieStoreOption, HttpSession, RedirectPolicy};
use httpsession::header::{ContentLength, UserAgent};
use pbr::{MultiBar, Units};
use regex::Regex;
use zip::ZipArchive;
use zip::result::{ZipError, ZipResult};

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

/// Constructs a `HttpSession`.
fn start_session(filename: &'static str, domain: &'static str) -> httpsession::Result<HttpSession> {
    HttpSession::builder()
        .base(domain, true, None)
        .cookie_store(CookieStoreOption::AutoSave(util::path_under_home(&[
            ".local",
            "share",
            "snowchains",
            filename,
        ])?))
        .echo_actions(ColorMode::Prefer256.disable_on("NO_COLOR"))
        .timeout(Duration::from_secs(20))
        .redirect(RedirectPolicy::none())
        .default_header(UserAgent::new(
            "snowchains <https://github.com/wariuni/snowchains>",
        ))
        .with_robots_txt()
}

/// Reads username and password from stdin, showing the prompts on stderr.
///
/// If fails to read a password because of OS error 6 or 32, askes a password again without hiding
/// the input.
fn ask_username_and_password(username_prompt: &'static str) -> io::Result<(String, String)> {
    let errno_brokenpipe = if cfg!(target_os = "windows") { 6 } else { 32 };
    let username = rprompt::prompt_reply_stderr(username_prompt)?;
    let password =
        rpassword::prompt_password_stderr("Password: ").or_else(|e| match e.raw_os_error() {
            Some(n) if n == errno_brokenpipe => {
                eprintln_bold!(Color::Warning, "os error {}", n);
                rprompt::prompt_reply_stderr("Password (not hidden): ")
            }
            _ => Err(e),
        })?;
    Ok((username, password))
}

/// Gets the value `x` if `Some(x) = o` and `!f(x)`.
///
/// # Errors
///
/// Returns `Err` if the above condition is not satisfied.
fn quit_on_failure<T>(o: Option<T>, f: for<'a> fn(&'a T) -> bool) -> ServiceResult<T> {
    if let Some(x) = o {
        if !f(&x) {
            return Ok(x);
        }
    }
    bail!(ServiceErrorKind::Scrape);
}

/// Reads a source code from `path`, replacing the main class name with `class_name` if the source
/// code is Java or Scala.
fn replace_class_name_if_necessary(path: &Path, class_name: &'static str) -> ServiceResult<String> {
    let replace = move |file: File, regex: &Regex, stem: &OsStr| -> io::Result<Option<String>> {
        let code = BufReader::new(file);
        let mut processed = vec![];
        let mut replaced_p = false;
        for line in code.lines() {
            let line = line?;
            if !replaced_p {
                if let Some(caps) = regex.captures(&line) {
                    if OsStr::new(&caps[2]) == stem {
                        processed.push(format!("{}{}{}", &caps[1], class_name, &caps[3]));
                        replaced_p = true;
                        continue;
                    }
                }
            }
            processed.push(line);
        }

        Ok(if replaced_p {
            info!(
                "The main class name was successfully replaced with {:?}",
                class_name
            );
            Some(processed.join("\n"))
        } else {
            None
        })
    };

    lazy_static! {
        static ref JAVA_CLASS: Regex =
            Regex::new(r"^(\s*public\s+class\s+)([a-zA-Z_\$][a-zA-Z0-9_\$]*)(.*)$").unwrap();
        static ref SCALA_CLASS: Regex =
            Regex::new(r"^(\s*object\s+)([a-zA-Z_\$][a-zA-Z0-9_\$]*)(.*)$").unwrap();
    }
    let file = util::open_file(path)?;
    let stem = path.file_stem().unwrap_or_default();
    let extension = path.extension();
    let e = || ServiceError::from(ServiceErrorKind::ClassNameReplace(path.to_owned()));
    if extension == Some(OsStr::new("java")) {
        replace(file, &JAVA_CLASS, stem)?.ok_or_else(e)
    } else if extension == Some(OsStr::new("scala")) {
        replace(file, &SCALA_CLASS, stem)?.ok_or_else(e)
    } else {
        Ok(util::string_from_read(file)?)
    }
}

trait OpenInBrowser {
    /// Opens `url`, which is relative or absolute, with default browser printing a message.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the exit status code is not 0 or an IO error occures.
    fn open_in_browser(&self, url: &str) -> ServiceResult<()>;
}

impl OpenInBrowser for HttpSession {
    fn open_in_browser(&self, url: &str) -> ServiceResult<()> {
        let url = self.resolve_url(url)?;
        println!("Opening {} in default browser...", url);
        let status = webbrowser::open(url.as_str())?.status;
        if !status.success() {
            bail!(ServiceErrorKind::Webbrowser(status));
        }
        Ok(())
    }
}

trait DownloadZips {
    /// Sends GET requests to `urls` expecting the response data are all zips.
    ///
    /// # Errors
    ///
    /// Fails if:
    /// - Any of `urls` is invalid
    /// - Any of http access fails
    /// - Any of response code is not 200
    /// - Any of downloaded zip is invalid
    fn download_zips(
        &mut self,
        urls: &[String],
        each_capacity_on_content_length_missing: usize,
    ) -> ServiceResult<Vec<ZipArchive<Cursor<Vec<u8>>>>>;
}

impl DownloadZips for HttpSession {
    fn download_zips(
        &mut self,
        urls: &[String],
        each_capacity_on_content_length_missing: usize,
    ) -> ServiceResult<Vec<ZipArchive<Cursor<Vec<u8>>>>> {
        let responses = urls.iter()
            .map(|url| self.get(url))
            .collect::<Result<Vec<_>, _>>()?;

        println!("Downloading zip files...");
        let mut mb = MultiBar::new();
        let handles = responses
            .into_iter()
            .map(|mut response| {
                let content_length = response.headers().get::<ContentLength>().map(|l| **l);
                let mut pb = mb.create_bar(content_length.unwrap_or(0));
                pb.set_units(Units::Bytes);
                thread::spawn(move || -> ZipResult<_> {
                    fn error() -> ZipError {
                        io::Error::new(io::ErrorKind::Other, "Read failed").into()
                    }

                    let mut cursor = Cursor::new(Vec::with_capacity(match content_length {
                        Some(content_length) => content_length as usize,
                        None => each_capacity_on_content_length_missing,
                    }));
                    let mut buf = [0; 10 * 1024];
                    loop {
                        let num_read_bytes = response.read(&mut buf)?;
                        if num_read_bytes == 0 {
                            pb.finish();
                            break ZipArchive::new(cursor);
                        }
                        if num_read_bytes > 10 * 1024 {
                            return Err(error());
                        }
                        let num_written_bytes = cursor.write(&buf[0..num_read_bytes])?;
                        if num_read_bytes != num_written_bytes {
                            return Err(error());
                        }
                        let position = cursor.position();
                        if content_length.is_none() {
                            pb.total = position;
                        }
                        pb.set(position);
                    }
                })
            })
            .collect::<Vec<_>>();
        mb.listen();

        let mut zips = vec![];
        for handle in handles {
            zips.push(match handle.join() {
                Ok(zip) => zip,
                Err(_) => bail!(ServiceErrorKind::Thread),
            }?);
        }
        Ok(zips)
    }
}
