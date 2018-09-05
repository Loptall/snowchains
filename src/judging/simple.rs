use command::JudgingCommand;
use console::{ConsoleWrite, Palette};
use errors::JudgeResult;
use judging::text::{Line, PrintAligned, Text, Width, Word};
use judging::{MillisRoundedUp, Outcome};
use testsuite::{ExpectedStdout, SimpleCase};
use util;

use diff;

use std::io::{self, Write as _Write};
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{self, cmp, fmt, thread};

pub(super) fn judge(case: &SimpleCase, solver: &Arc<JudgingCommand>) -> JudgeResult<SimpleOutcome> {
    let (tx, rx) = std::sync::mpsc::channel();
    {
        let (solver, input) = (solver.clone(), case.input());
        thread::spawn(move || {
            let _ = tx.send(run_command(&solver, &input));
        });
    }
    let tle = |timelimit: Duration| SimpleOutcome::TimelimitExceeded {
        timelimit,
        input: Text::exact(&case.input()),
        expected: match case.expected().as_ref() {
            ExpectedStdout::AcceptAny { .. } => None,
            ExpectedStdout::Exact(expected) => Some(Text::exact(expected)),
            ExpectedStdout::Float {
                lines: expected, ..
            } => Some(Text::float(expected, None)),
        },
    };
    let outcome;
    if let Some(timelimit) = case.timelimit() {
        match rx.recv_timeout(timelimit + Duration::from_millis(50)) {
            Ok(o) => outcome = o?,
            Err(_) => return Ok(tle(timelimit)),
        }
        if outcome.elapsed > timelimit {
            return Ok(tle(timelimit));
        }
    } else {
        outcome = rx.recv().unwrap()?;
    }
    Ok(outcome.compare(&case.expected()))
}

fn run_command(solver: &JudgingCommand, input: &Arc<String>) -> JudgeResult<CommandOutcome> {
    let mut solver = solver.spawn_piped()?;
    let start = Instant::now();
    solver.stdin.as_mut().unwrap().write_all(input.as_bytes())?;

    let status = solver.wait()?;
    let elapsed = start.elapsed();
    let stdout = Arc::new(util::string_from_read(solver.stdout.unwrap(), 1024)?);
    let stderr = Arc::new(util::string_from_read(solver.stderr.unwrap(), 1024)?);
    Ok(CommandOutcome {
        status,
        elapsed,
        input: input.clone(),
        stdout,
        stderr,
    })
}

struct CommandOutcome {
    status: ExitStatus,
    elapsed: Duration,
    input: Arc<String>,
    stdout: Arc<String>,
    stderr: Arc<String>,
}

impl CommandOutcome {
    fn compare(&self, expected: &ExpectedStdout) -> SimpleOutcome {
        let (status, elapsed) = (self.status, self.elapsed);
        let input = Text::exact(&self.input);
        let stderr = Text::exact(&self.stderr);
        let (stdout, expected) = match expected {
            ExpectedStdout::AcceptAny => (Text::exact(&self.stdout), None),
            ExpectedStdout::Exact(expected) => {
                (Text::exact(&self.stdout), Some(Text::exact(expected)))
            }
            ExpectedStdout::Float {
                lines,
                absolute_error,
                relative_error,
            } => {
                let errors = Some((*absolute_error, *relative_error));
                let expected = Text::float(lines, errors);
                let stdout = Text::float(&self.stdout, None);
                (stdout, Some(expected))
            }
        };
        if !status.success() {
            SimpleOutcome::RuntimeError {
                elapsed,
                input,
                expected,
                stdout,
                stderr,
                status,
            }
        } else if expected.is_some() && *expected.as_ref().unwrap() != stdout {
            SimpleOutcome::WrongAnswer {
                elapsed,
                input,
                diff: TextDiff::new(expected.as_ref().unwrap(), &stdout),
                stderr,
            }
        } else {
            SimpleOutcome::Accepted {
                elapsed,
                input,
                stdout,
                stderr,
            }
        }
    }
}

/// Test result.
#[cfg_attr(test, derive(Debug))]
pub(super) enum SimpleOutcome {
    Accepted {
        elapsed: Duration,
        input: Text,
        stdout: Text,
        stderr: Text,
    },
    TimelimitExceeded {
        timelimit: Duration,
        expected: Option<Text>,
        input: Text,
    },
    WrongAnswer {
        elapsed: Duration,
        input: Text,
        diff: TextDiff,
        stderr: Text,
    },
    RuntimeError {
        elapsed: Duration,
        input: Text,
        expected: Option<Text>,
        stdout: Text,
        stderr: Text,
        status: ExitStatus,
    },
}

impl fmt::Display for SimpleOutcome {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SimpleOutcome::Accepted { elapsed, .. } => {
                write!(f, "Accepted ({}ms)", elapsed.millis_rounded_up())
            }
            SimpleOutcome::TimelimitExceeded { timelimit, .. } => write!(
                f,
                "Time Limit Exceeded ({}ms)",
                timelimit.millis_rounded_up()
            ),
            SimpleOutcome::WrongAnswer { elapsed, .. } => {
                write!(f, "Wrong Answer ({}ms)", elapsed.millis_rounded_up())
            }
            SimpleOutcome::RuntimeError {
                elapsed, status, ..
            } => {
                let elapsed = elapsed.millis_rounded_up();
                write!(f, "Runtime Error ({}, {}ms)", status, elapsed)
            }
        }
    }
}

impl Outcome for SimpleOutcome {
    fn failure(&self) -> bool {
        match self {
            SimpleOutcome::Accepted { .. } => false,
            _ => true,
        }
    }

    fn palette(&self) -> Palette {
        match self {
            SimpleOutcome::Accepted { .. } => Palette::Success,
            SimpleOutcome::TimelimitExceeded { .. } => Palette::Fatal,
            SimpleOutcome::WrongAnswer { .. } | SimpleOutcome::RuntimeError { .. } => {
                Palette::Warning
            }
        }
    }

    fn print_details(&self, mut out: impl ConsoleWrite) -> io::Result<()> {
        fn print_section(mut out: impl ConsoleWrite, title: &str, text: &Text) -> io::Result<()> {
            writeln!(out.bold(Palette::Title), "{}", title)?;
            if text.is_empty() {
                writeln!(out.bold(Palette::Warning), "EMPTY")
            } else {
                text.print_all(out)
            }
        }

        fn print_section_unless_empty<'a>(
            mut out: impl ConsoleWrite,
            title: &str,
            text: impl Into<Option<&'a Text>>,
        ) -> io::Result<()> {
            if let Some(text) = text.into() {
                if !text.is_empty() {
                    writeln!(out.bold(Palette::Title), "{}", title)?;
                    text.print_all(out)?;
                }
            }
            Ok(())
        }

        fn print_diff(mut out: impl ConsoleWrite, title: &str, diff: &TextDiff) -> io::Result<()> {
            writeln!(out.bold(Palette::Title), "{}", title)?;
            diff.print(out)
        }

        match self {
            SimpleOutcome::Accepted {
                input,
                stdout,
                stderr,
                ..
            } => {
                print_section(&mut out, "input:", input)?;
                print_section(&mut out, "stdout:", stdout)?;
                print_section_unless_empty(&mut out, "stderr:", stderr)
            }
            SimpleOutcome::TimelimitExceeded {
                input, expected, ..
            } => {
                print_section(&mut out, "input:", input)?;
                print_section_unless_empty(&mut out, "expected:", expected.as_ref())
            }
            SimpleOutcome::WrongAnswer {
                input,
                diff,
                stderr,
                ..
            } => {
                print_section(&mut out, "input:", input)?;
                print_diff(&mut out, "diff:", diff)?;
                print_section_unless_empty(&mut out, "stderr:", stderr)
            }
            SimpleOutcome::RuntimeError {
                input,
                expected,
                stdout,
                stderr,
                ..
            } => {
                print_section(&mut out, "input:", input)?;
                print_section_unless_empty(&mut out, "expected:", expected.as_ref())?;
                print_section_unless_empty(&mut out, "stdout:", stdout)?;
                print_section(&mut out, "stderr:", stderr)
            }
        }
    }
}

#[cfg_attr(test, derive(Debug))]
pub(super) enum TextDiff {
    SameNumLines {
        lines: Vec<(LineDiffDetialed, LineDiffDetialed)>,
    },
    Lines {
        lines: Vec<(LineDiff, LineDiff)>,
    },
}

impl TextDiff {
    fn new(left: &Text, right: &Text) -> Self {
        if left.lines().len() == right.lines().len() {
            let mut lines = vec![];
            for (left, right) in left.lines().iter().zip(right.lines()) {
                let (mut l_diffs, mut r_diffs) = (vec![], vec![]);
                for diff in diff::slice(left.words(), right.words()) {
                    match diff {
                        diff::Result::Left(l) => l_diffs.push(Diff::NotCommon(l.clone())),
                        diff::Result::Right(r) => r_diffs.push(Diff::NotCommon(r.clone())),
                        diff::Result::Both(l, r) => {
                            l_diffs.push(Diff::Common(l.clone()));
                            r_diffs.push(Diff::Common(r.clone()));
                        }
                    }
                }
                lines.push((Line::new(l_diffs), Line::new(r_diffs)));
            }
            TextDiff::SameNumLines { lines }
        } else {
            #[derive(Default)]
            struct St {
                lines: Vec<(LineDiff, LineDiff)>,
                l_diffs: Vec<Line<Word>>,
                r_diffs: Vec<Line<Word>>,
            }

            impl St {
                fn clean_up(&mut self) {
                    let (l_diffs_len, r_diffs_len) = (self.l_diffs.len(), self.r_diffs.len());
                    for i in 0..cmp::min(l_diffs_len, r_diffs_len) {
                        let left = Diff::NotCommon(self.l_diffs[i].clone());
                        let right = Diff::NotCommon(self.r_diffs[i].clone());
                        self.lines.push((left, right));
                    }
                    let empty = Diff::NotCommon(Line::default());
                    if l_diffs_len < r_diffs_len {
                        for i in l_diffs_len..r_diffs_len {
                            let right = Diff::NotCommon(self.r_diffs[i].clone());
                            self.lines.push((empty.clone(), right));
                        }
                    } else {
                        for i in r_diffs_len..l_diffs_len {
                            let left = Diff::NotCommon(self.l_diffs[i].clone());
                            self.lines.push((left, empty.clone()));
                        }
                    }
                    self.l_diffs.clear();
                    self.r_diffs.clear();
                }
            }

            let mut st = St::default();
            for diff in diff::slice(left.lines(), right.lines()) {
                match diff {
                    diff::Result::Left(l) => st.l_diffs.push(l.clone()),
                    diff::Result::Right(r) => st.r_diffs.push(r.clone()),
                    diff::Result::Both(l, r) => {
                        st.clean_up();
                        st.lines
                            .push((Diff::Common(l.clone()), Diff::Common(r.clone())));
                    }
                }
            }
            st.clean_up();
            TextDiff::Lines { lines: st.lines }
        }
    }

    fn print(&self, out: impl ConsoleWrite) -> io::Result<()> {
        fn print(
            lines: &[(impl PrintAligned, impl PrintAligned)],
            mut out: impl ConsoleWrite,
        ) -> io::Result<()> {
            let (l_max_width, r_max_width) = {
                let (mut l_max_width, mut r_max_width) = (0, 0);
                for (l, r) in lines {
                    l_max_width = cmp::max(l_max_width, l.width(out.str_width_fn()));
                    r_max_width = cmp::max(r_max_width, r.width(out.str_width_fn()));
                }
                (l_max_width, r_max_width)
            };
            let (wl, wr) = (cmp::max(l_max_width, 8), cmp::max(r_max_width, 6));
            out.write_all("│".as_bytes())?;
            out.bold(Palette::Title).write_all(b"expected")?;
            out.write_spaces(wl - 8)?;
            out.write_all("│".as_bytes())?;
            out.bold(Palette::Title).write_all(b"stdout")?;
            out.write_spaces(wr - 6)?;
            out.write_all("│\n".as_bytes())?;
            for (l, r) in lines {
                out.write_all("│".as_bytes())?;
                l.print_aligned(&mut out, wl)?;
                out.write_all("│".as_bytes())?;
                r.print_aligned(&mut out, wr)?;
                out.write_all("│\n".as_bytes())?;
            }
            Ok(())
        }

        match self {
            TextDiff::SameNumLines { lines } => print(lines, out),
            TextDiff::Lines { lines } => print(lines, out),
        }
    }
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub(super) enum Diff<T> {
    Common(T),
    NotCommon(T),
}

impl<T: Width> Width for Diff<T> {
    fn width(&self, f: fn(&str) -> usize) -> usize {
        match self {
            Diff::Common(x) => x.width(f),
            Diff::NotCommon(x) => x.width(f),
        }
    }
}

type LineDiffDetialed = Line<Diff<Word>>;
type LineDiff = Diff<Line<Word>>;

impl PrintAligned for LineDiffDetialed {
    fn print_aligned<W: ConsoleWrite>(&self, mut out: W, min_width: usize) -> io::Result<()> {
        for word_diff in self.words() {
            match word_diff {
                Diff::Common(w) => w.print_as_common(&mut out),
                Diff::NotCommon(w) => w.print_as_difference(&mut out),
            }?;
        }
        let width = self.width(out.str_width_fn());
        out.write_spaces(cmp::max(width, min_width) - width)
    }
}

impl PrintAligned for LineDiff {
    fn print_aligned<W: ConsoleWrite>(&self, mut out: W, min_width: usize) -> io::Result<()> {
        let (l, f): (_, fn(&Word, &mut W) -> io::Result<()>) = match self {
            Diff::Common(l) => (l, |w, out| w.print_as_common(out)),
            Diff::NotCommon(l) => (l, |w, out| w.print_as_difference(out)),
        };
        l.words().iter().try_for_each(|w| f(w, &mut out))?;
        let width = l.width(out.str_width_fn());
        out.write_spaces(cmp::max(width, min_width) - width)
    }
}

impl Text {
    fn float(s: &str, errors: Option<(f64, f64)>) -> Self {
        if let Some((absolute_error, relative_error)) = errors {
            let on_plain = |string: String| match string.parse::<f64>() {
                Ok(value) => Word::FloatLeft {
                    value,
                    string: Arc::new(string),
                    absolute_error,
                    relative_error,
                },
                Err(_) => Word::Plain(Arc::new(string)),
            };
            Self::new(s, on_plain)
        } else {
            fn on_plain(string: String) -> Word {
                let string = Arc::new(string);
                match string.parse::<f64>() {
                    Ok(value) => Word::FloatRight { value, string },
                    Err(_) => Word::Plain(string),
                }
            }
            Self::new(s, on_plain)
        }
    }

    fn print_all(&self, mut out: impl ConsoleWrite) -> io::Result<()> {
        for line in self.lines() {
            for word in line.words() {
                word.print_as_common(&mut out)?;
            }
            writeln!(out)?;
        }
        Ok(())
    }
}
