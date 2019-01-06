#![allow(dead_code)]

use snowchains::app::{App, Opt};
use snowchains::path::{AbsPath, AbsPathBuf};
use snowchains::service::{Credentials, RevelSession, ServiceName, UserNameAndPassword};
use snowchains::terminal::{AnsiColorChoice, TermImpl};

use failure::Fallible;
use if_chain::if_chain;
use serde_derive::Deserialize;
use serde_json::json;
use tempdir::TempDir;

use std::fs::File;
use std::panic::UnwindSafe;
use std::path::PathBuf;
use std::{env, io, panic};

pub fn test_in_tempdir<E: Into<failure::Error>>(
    tempdir_prefix: &str,
    credentials: Credentials,
    f: impl FnOnce(App<TermImpl<io::Empty, io::Sink, io::Sink>>) -> Result<(), E> + UnwindSafe,
) -> Fallible<()> {
    let tempdir = TempDir::new(tempdir_prefix).unwrap();
    let tempdir_path = tempdir.path().to_owned();
    let result = panic::catch_unwind(move || -> Fallible<()> {
        let mut app = App {
            working_dir: AbsPathBuf::try_new(&tempdir_path).unwrap(),
            cookies_on_init: "$service".to_owned(),
            dropbox_auth_on_init: "dropbox.json".to_owned(),
            enable_dropbox_on_init: true,
            credentials,
            term: TermImpl::null(),
        };
        app.run(Opt::Init {
            color_choice: AnsiColorChoice::Never,
            directory: PathBuf::from("."),
        })?;
        serde_json::to_writer(
            File::create(tempdir_path.join("dropbox.json"))?,
            &json!({ "access_token": env("DROPBOX_ACCESS_TOKEN")? }),
        )?;
        f(app).map_err(Into::into)
    });
    tempdir.close().unwrap();
    match result {
        Err(panic) => panic::resume_unwind(panic),
        Ok(result) => result,
    }
}

pub fn credentials_from_env_vars() -> Fallible<Credentials> {
    let atcoder_username = env("ATCODER_USERNAME")?;
    let atcoder_password = env("ATCODER_PASSWORD")?;
    let yukicoder_revel_session = env("YUKICODER_REVEL_SESSION")?;
    Ok(Credentials {
        atcoder: UserNameAndPassword::Some(atcoder_username, atcoder_password),
        yukicoder: RevelSession::Some(yukicoder_revel_session),
    })
}

pub fn dummy_credentials() -> Credentials {
    let dummy = "".to_owned();
    Credentials {
        atcoder: UserNameAndPassword::Some(dummy.clone(), dummy.clone()),
        yukicoder: RevelSession::Some(dummy),
    }
}

fn env(name: &'static str) -> Fallible<String> {
    let output = std::process::Command::new("envchain")
        .args(&["snowchains", "sh", "-c"])
        .arg(format!("printf %s ${}", name))
        .output();
    if_chain! {
        if let Ok(std::process::Output { status, stdout, .. }) = output;
        if status.success() && !stdout.is_empty();
        if let Ok(stdout) = String::from_utf8(stdout);
        then {
            Ok(stdout)
        } else {
            env::var(name)
                .map_err(|e| failure::err_msg(format!("Failed to read {:?}: {}", name, e)))
        }
    }
}

pub fn login(
    mut app: App<TermImpl<io::Empty, io::Sink, io::Sink>>,
    service: ServiceName,
) -> snowchains::Result<()> {
    app.run(Opt::Login {
        color_choice: AnsiColorChoice::Never,
        service,
    })
}

pub fn download(
    mut app: App<TermImpl<io::Empty, io::Sink, io::Sink>>,
    service: ServiceName,
    contest: &str,
    problems: &[&str],
) -> snowchains::Result<()> {
    app.run(Opt::Download {
        open: false,
        only_scraped: false,
        service: Some(service),
        contest: Some(contest.to_owned()),
        problems: problems.iter().map(|&s| s.to_owned()).collect(),
        color_choice: AnsiColorChoice::Never,
    })
}

pub fn confirm_num_cases(
    wd: &AbsPath,
    service: ServiceName,
    contest: &str,
    pairs: &[(&str, usize)],
) {
    #[derive(Deserialize)]
    struct SimpleSuite {
        cases: Vec<serde_yaml::Value>,
    }

    for &(problem, expected_num_cases) in pairs {
        let path = wd
            .join("tests")
            .join(<&str>::from(service))
            .join(contest)
            .join(format!("{}.yaml", problem));
        let file = File::open(&path).unwrap();
        let suite = serde_yaml::from_reader::<_, SimpleSuite>(file).unwrap();
        assert_eq!(expected_num_cases, suite.cases.len());
    }
}