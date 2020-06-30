pub fn ask<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .help("Path to Threshfile")
                .takes_value(true)
                .default_value(".threshfile"),
        )
        .arg(
            clap::Arg::with_name("secret")
                .short("s")
                .long("secret")
                .help("Secret to generate and authenticate the token. Can also be provided in the Threshfile")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Makes Thresh verbose. Useful for debugging and seeing what's going on \"under the hood\"")
        )
        .subcommand(
            clap::App::new("serve")
                .about("Start thresh agent")
                .arg(
                    clap::Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .help("Custom server port")
                        .takes_value(true),
                )
                .arg(
                    clap::Arg::with_name("logs-dir")
                        .short("l")
                        .long("logs-dir")
                        .help("Custom logs directory")
                        .takes_value(true),
                ),
        )
        .subcommand(
            clap::App::new("token")
                .about(
                    "Create a token based on the secret to authorize agent connections",
                )
        )
}