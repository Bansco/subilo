pub fn ask<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .help("Path to Subilofile")
                .takes_value(true)
                .default_value(".subilofile"),
        )
        .arg(
            clap::Arg::with_name("secret")
                .short("s")
                .long("secret")
                .help("Secret to generate and authenticate the token. Can also be provided in the Subilofile")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Makes Subilo verbose. Useful for debugging and seeing what's going on \"under the hood\"")
        )
        .subcommand(
            clap::App::new("serve")
                .about("Start subilo agent")
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
                .about("Create a token based on the secret to authorize agent connections")
                .arg(
                    clap::Arg::with_name("permissions")
                        .short("p")
                        .long("permissions")
                        .help("Token permissions")
                        .default_value("job:write")
                        .takes_value(true),
                )
                .arg(
                    clap::Arg::with_name("duration")
                        .short("d")
                        .long("duration")
                        .help("Token duration until expires in minutes")
                        .default_value("43800")
                        .takes_value(true),
                )
        )
}
