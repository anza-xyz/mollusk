use {
    clap::{
        error::{Error, ErrorKind},
        Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches,
    },
    mollusk_svm::Mollusk,
    solana_pubkey::Pubkey,
    std::str::FromStr,
};

const ADD_PROGRAM_ID: &str = "add-program";
const ADD_PROGRAM_WITH_LOADER_ID: &str = "add-program-with-loader";
const ADD_PROGRAM_WITH_LOADER_AND_ELF_ID: &str = "add-program-with-loader-and-elf";

#[derive(Clone, Debug, PartialEq, Eq)]
struct AddProgramArg {
    program_id: Pubkey,
    program_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AddProgramWithLoaderArg {
    program_id: Pubkey,
    program_name: String,
    loader_key: Pubkey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AddProgramWithLoaderAndElfArg {
    program_id: Pubkey,
    loader_key: Pubkey,
    elf_path: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramLoadArgs {
    add_program: Vec<AddProgramArg>,
    add_program_with_loader: Vec<AddProgramWithLoaderArg>,
    add_program_with_loader_and_elf: Vec<AddProgramWithLoaderAndElfArg>,
}

impl ProgramLoadArgs {
    fn build_args(cmd: Command) -> Command {
        cmd.arg(
            Arg::new(ADD_PROGRAM_ID)
                .long(ADD_PROGRAM_ID)
                .help("Add a program by name using Mollusk's default loader")
                .num_args(2)
                .action(ArgAction::Append)
                .value_names(["PROGRAM_ID", "PROGRAM_NAME"])
                .value_parser(clap::value_parser!(String))
                .next_line_help(true),
        )
        .arg(
            Arg::new(ADD_PROGRAM_WITH_LOADER_ID)
                .long(ADD_PROGRAM_WITH_LOADER_ID)
                .help("Add a program by name under an explicit loader")
                .num_args(3)
                .action(ArgAction::Append)
                .value_names(["PROGRAM_ID", "PROGRAM_NAME", "LOADER_KEY"])
                .value_parser(clap::value_parser!(String))
                .next_line_help(true),
        )
        .arg(
            Arg::new(ADD_PROGRAM_WITH_LOADER_AND_ELF_ID)
                .long(ADD_PROGRAM_WITH_LOADER_AND_ELF_ID)
                .help("Add a program from an explicit ELF path under an explicit loader")
                .num_args(3)
                .action(ArgAction::Append)
                .value_names(["PROGRAM_ID", "LOADER_KEY", "ELF_PATH"])
                .value_parser(clap::value_parser!(String))
                .value_hint(clap::ValueHint::FilePath)
                .next_line_help(true),
        )
    }
}

impl FromArgMatches for ProgramLoadArgs {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error> {
        let mut matches = matches.clone();
        Self::from_arg_matches_mut(&mut matches)
    }

    fn from_arg_matches_mut(matches: &mut ArgMatches) -> Result<Self, Error> {
        Ok(Self {
            add_program: parse_add_program_args(matches)?,
            add_program_with_loader: parse_add_program_with_loader_args(matches)?,
            add_program_with_loader_and_elf: parse_add_program_with_loader_and_elf_args(matches)?,
        })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), Error> {
        let mut matches = matches.clone();
        self.update_from_arg_matches_mut(&mut matches)
    }

    fn update_from_arg_matches_mut(&mut self, matches: &mut ArgMatches) -> Result<(), Error> {
        *self = Self::from_arg_matches_mut(matches)?;
        Ok(())
    }
}

impl Args for ProgramLoadArgs {
    fn augment_args(cmd: Command) -> Command {
        Self::build_args(cmd)
    }

    fn augment_args_for_update(cmd: Command) -> Command {
        Self::build_args(cmd)
    }
}

fn parse_pubkey(value: String, name: &str) -> Result<Pubkey, Error> {
    Pubkey::from_str(&value).map_err(|err| {
        Error::raw(
            ErrorKind::ValueValidation,
            format!("invalid {name} `{value}`: {err}"),
        )
    })
}

/// Remove one repeatable multi-value flag from `ArgMatches`, returning each
/// flag occurrence as its own `Vec<String>`.
///
/// For example, `--add-program A spl_token --add-program B spl_token_2022`
/// becomes `vec![vec!["A", "spl_token"], vec!["B", "spl_token_2022"]]`.
fn remove_string_occurrences(matches: &mut ArgMatches, id: &str) -> Vec<Vec<String>> {
    matches
        .remove_occurrences::<String>(id)
        .map(|occurrences| occurrences.map(Iterator::collect).collect())
        .unwrap_or_default()
}

fn parse_add_program_args(matches: &mut ArgMatches) -> Result<Vec<AddProgramArg>, Error> {
    remove_string_occurrences(matches, ADD_PROGRAM_ID)
        .into_iter()
        .map(|chunk| {
            let mut values = chunk.into_iter();
            let program_id = parse_pubkey(values.next().unwrap(), "program id")?;
            let program_name = values.next().unwrap();
            Ok(AddProgramArg {
                program_id,
                program_name,
            })
        })
        .collect()
}

fn parse_add_program_with_loader_args(
    matches: &mut ArgMatches,
) -> Result<Vec<AddProgramWithLoaderArg>, Error> {
    remove_string_occurrences(matches, ADD_PROGRAM_WITH_LOADER_ID)
        .into_iter()
        .map(|chunk| {
            let mut values = chunk.into_iter();
            let program_id = parse_pubkey(values.next().unwrap(), "program id")?;
            let program_name = values.next().unwrap();
            let loader_key = parse_pubkey(values.next().unwrap(), "loader key")?;
            Ok(AddProgramWithLoaderArg {
                program_id,
                program_name,
                loader_key,
            })
        })
        .collect()
}

fn parse_add_program_with_loader_and_elf_args(
    matches: &mut ArgMatches,
) -> Result<Vec<AddProgramWithLoaderAndElfArg>, Error> {
    remove_string_occurrences(matches, ADD_PROGRAM_WITH_LOADER_AND_ELF_ID)
        .into_iter()
        .map(|chunk| {
            let mut values = chunk.into_iter();
            let program_id = parse_pubkey(values.next().unwrap(), "program id")?;
            let loader_key = parse_pubkey(values.next().unwrap(), "loader key")?;
            let elf_path = values.next().unwrap();
            Ok(AddProgramWithLoaderAndElfArg {
                program_id,
                loader_key,
                elf_path,
            })
        })
        .collect()
}

/// Apply any optional CLI-specified program preloads to a Mollusk instance.
pub fn apply_program_load_args(mollusk: &mut Mollusk, program_load_args: &ProgramLoadArgs) {
    for spec in &program_load_args.add_program {
        mollusk.add_program(&spec.program_id, &spec.program_name);
    }

    for spec in &program_load_args.add_program_with_loader {
        mollusk.add_program_with_loader(&spec.program_id, &spec.program_name, &spec.loader_key);
    }

    for spec in &program_load_args.add_program_with_loader_and_elf {
        let elf = mollusk_svm::file::read_file(&spec.elf_path);
        mollusk.add_program_with_loader_and_elf(&spec.program_id, &spec.loader_key, &elf);
    }
}

#[cfg(test)]
mod tests {
    use {
        super::{
            AddProgramArg, AddProgramWithLoaderAndElfArg, AddProgramWithLoaderArg, ProgramLoadArgs,
        },
        clap::{Parser, Subcommand},
        solana_pubkey::Pubkey,
        std::str::FromStr,
    };

    const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
    const BPF_LOADER_V2: &str = "BPFLoader2111111111111111111111111111111111";
    const BPF_LOADER_V3: &str = "BPFLoaderUpgradeab1e11111111111111111111111";

    #[derive(Debug, Subcommand)]
    enum TestSubcommand {
        Test {
            #[command(flatten)]
            program_load_args: ProgramLoadArgs,
        },
    }

    #[derive(Debug, Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: TestSubcommand,
    }

    #[test]
    fn parses_add_program_flag_occurrence() {
        let cli = TestCli::try_parse_from([
            "mollusk",
            "test",
            "--add-program",
            TOKEN_PROGRAM_ID,
            "spl_token",
        ])
        .unwrap();

        let TestSubcommand::Test { program_load_args } = cli.command;
        assert_eq!(
            program_load_args,
            ProgramLoadArgs {
                add_program: vec![AddProgramArg {
                    program_id: Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap(),
                    program_name: "spl_token".to_string(),
                }],
                ..ProgramLoadArgs::default()
            }
        );
    }

    #[test]
    fn parses_multiple_add_program_flag_occurrences() {
        let cli = TestCli::try_parse_from([
            "mollusk",
            "test",
            "--add-program",
            TOKEN_PROGRAM_ID,
            "spl_token",
            "--add-program",
            TOKEN_2022_PROGRAM_ID,
            "spl_token_2022",
        ])
        .unwrap();

        let TestSubcommand::Test { program_load_args } = cli.command;
        assert_eq!(
            program_load_args,
            ProgramLoadArgs {
                add_program: vec![
                    AddProgramArg {
                        program_id: Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap(),
                        program_name: "spl_token".to_string(),
                    },
                    AddProgramArg {
                        program_id: Pubkey::from_str(TOKEN_2022_PROGRAM_ID).unwrap(),
                        program_name: "spl_token_2022".to_string(),
                    },
                ],
                ..ProgramLoadArgs::default()
            }
        );
    }

    #[test]
    fn rejects_incomplete_add_program_flag_occurrence() {
        assert!(
            TestCli::try_parse_from(["mollusk", "test", "--add-program", TOKEN_PROGRAM_ID])
                .is_err()
        );
    }

    #[test]
    fn parses_add_program_with_loader_flag_occurrence() {
        let cli = TestCli::try_parse_from([
            "mollusk",
            "test",
            "--add-program-with-loader",
            TOKEN_PROGRAM_ID,
            "spl_token",
            BPF_LOADER_V2,
        ])
        .unwrap();

        let TestSubcommand::Test { program_load_args } = cli.command;
        assert_eq!(
            program_load_args,
            ProgramLoadArgs {
                add_program_with_loader: vec![AddProgramWithLoaderArg {
                    program_id: Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap(),
                    program_name: "spl_token".to_string(),
                    loader_key: Pubkey::from_str(BPF_LOADER_V2).unwrap(),
                }],
                ..ProgramLoadArgs::default()
            }
        );
    }

    #[test]
    fn rejects_incomplete_add_program_with_loader_flag_occurrence() {
        assert!(TestCli::try_parse_from([
            "mollusk",
            "test",
            "--add-program-with-loader",
            TOKEN_PROGRAM_ID,
            "spl_token",
        ])
        .is_err());
    }

    #[test]
    fn parses_add_program_with_loader_and_elf_flag_occurrence() {
        let elf_path = "/tmp/token_2022.so";
        let cli = TestCli::try_parse_from([
            "mollusk",
            "test",
            "--add-program-with-loader-and-elf",
            TOKEN_2022_PROGRAM_ID,
            BPF_LOADER_V3,
            elf_path,
        ])
        .unwrap();

        let TestSubcommand::Test { program_load_args } = cli.command;
        assert_eq!(
            program_load_args,
            ProgramLoadArgs {
                add_program_with_loader_and_elf: vec![AddProgramWithLoaderAndElfArg {
                    program_id: Pubkey::from_str(TOKEN_2022_PROGRAM_ID).unwrap(),
                    loader_key: Pubkey::from_str(BPF_LOADER_V3).unwrap(),
                    elf_path: elf_path.to_string(),
                }],
                ..ProgramLoadArgs::default()
            }
        );
    }

    #[test]
    fn rejects_incomplete_add_program_with_loader_and_elf_flag_occurrence() {
        assert!(TestCli::try_parse_from([
            "mollusk",
            "test",
            "--add-program-with-loader-and-elf",
            TOKEN_2022_PROGRAM_ID,
            BPF_LOADER_V3,
        ])
        .is_err());
    }

    #[test]
    fn parses_mixed_program_loading_flags() {
        let elf_path = "/tmp/token_2022.so";
        let cli = TestCli::try_parse_from([
            "mollusk",
            "test",
            "--add-program",
            TOKEN_PROGRAM_ID,
            "spl_token",
            "--add-program-with-loader",
            TOKEN_PROGRAM_ID,
            "spl_token",
            BPF_LOADER_V2,
            "--add-program-with-loader-and-elf",
            TOKEN_2022_PROGRAM_ID,
            BPF_LOADER_V3,
            elf_path,
        ])
        .unwrap();

        let TestSubcommand::Test { program_load_args } = cli.command;
        assert_eq!(
            program_load_args,
            ProgramLoadArgs {
                add_program: vec![AddProgramArg {
                    program_id: Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap(),
                    program_name: "spl_token".to_string(),
                }],
                add_program_with_loader: vec![AddProgramWithLoaderArg {
                    program_id: Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap(),
                    program_name: "spl_token".to_string(),
                    loader_key: Pubkey::from_str(BPF_LOADER_V2).unwrap(),
                }],
                add_program_with_loader_and_elf: vec![AddProgramWithLoaderAndElfArg {
                    program_id: Pubkey::from_str(TOKEN_2022_PROGRAM_ID).unwrap(),
                    loader_key: Pubkey::from_str(BPF_LOADER_V3).unwrap(),
                    elf_path: elf_path.to_string(),
                }],
            }
        );
    }

    #[test]
    fn rejects_invalid_program_pubkey() {
        let err = TestCli::try_parse_from([
            "mollusk",
            "test",
            "--add-program",
            "not_a_pubkey",
            "spl_token",
        ])
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("invalid program id `not_a_pubkey`"));
    }
}
