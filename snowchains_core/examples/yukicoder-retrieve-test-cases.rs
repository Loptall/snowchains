use anyhow::Context as _;
use either::Either;
use snowchains_core::web::{
    RetrieveFullTestCases, RetrieveSampleTestCases, StandardStreamShell, Yukicoder,
    YukicoderRetrieveFullTestCasesCredentials,
};
use std::{env, str};
use structopt::StructOpt;
use strum::{EnumString, EnumVariantNames, VariantNames as _};
use termcolor::ColorChoice;

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(long)]
    full: bool,

    #[structopt(short, long, value_name("HUMANTIME"))]
    timeout: Option<humantime::Duration>,

    #[structopt(
        long,
        value_name("VIA"),
        default_value("prompt"),
        possible_values(CredentialsVia::VARIANTS)
    )]
    credentials: CredentialsVia,

    #[structopt(short, long, value_name("CONTEST_ID"))]
    contest: Option<String>,

    #[structopt(
        short,
        long,
        value_name("PROBLEM_NUMBERS_OR_SLUGS_IN_CONTEST"),
        required_unless("contest")
    )]
    problems: Option<Vec<String>>,
}

#[derive(EnumString, EnumVariantNames, Debug)]
#[strum(serialize_all = "kebab-case")]
enum CredentialsVia {
    Prompt,
    Env,
}

fn main() -> anyhow::Result<()> {
    let Opt {
        full,
        timeout,
        credentials,
        problems,
        contest,
    } = Opt::from_args();

    let targets = match (contest, problems) {
        (None, None) => unreachable!(),
        (None, Some(problem_nos)) => Either::Left(
            problem_nos
                .iter()
                .map(|p| p.parse::<u64>())
                .collect::<Result<Vec<_>, _>>()
                .with_context(|| "Problem numbers be unsigned integer")?,
        ),
        (Some(contest), problem_slugs) => Either::Right((contest, problem_slugs)),
    };

    let targets = match &targets {
        Either::Left(problem_nos) => Either::Left(&**problem_nos),
        Either::Right((contest, problem_slugs)) => {
            Either::Right((contest, problem_slugs.as_deref()))
        }
    };

    let timeout = timeout.map(Into::into);

    let shell = StandardStreamShell::new(if atty::is(atty::Stream::Stderr) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    });

    if full {
        Yukicoder::exec(RetrieveFullTestCases {
            targets,
            timeout,
            cookies: (),
            shell,
            credentials: YukicoderRetrieveFullTestCasesCredentials {
                api_key: || match credentials {
                    CredentialsVia::Prompt => {
                        rpassword::read_password_from_tty(Some("yukicoder API Key: "))
                            .map_err(Into::into)
                    }
                    CredentialsVia::Env => env::var("YUKICODER_API_KEY").map_err(Into::into),
                },
            },
        })?;
    } else {
        let outcome = Yukicoder::exec(RetrieveSampleTestCases {
            targets,
            timeout,
            cookies: (),
            shell,
            credentials: (),
        })?;

        dbg!(outcome);
    }

    Ok(())
}
