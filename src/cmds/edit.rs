//! Edit command
use super::Command;
use crate::{Error, Result};
use anyhow::anyhow;
use async_trait::async_trait;
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command as ClapCommand};
use std::collections::HashMap;

/// Abstract `edit` command
///
/// ```sh
/// leetcode-edit
/// Edit question by id
///
/// USAGE:
///     leetcode edit <id>
///
/// FLAGS:
///     -h, --help       Prints help information
///     -V, --version    Prints version information
///
/// ARGS:
///     <id>    question id
/// ```
pub struct EditCommand;

#[async_trait]
impl Command for EditCommand {
    /// `edit` usage
    fn usage() -> ClapCommand {
        ClapCommand::new("edit")
            .about("Edit question")
            .visible_alias("e")
            .arg(
                Arg::new("lang")
                    .short('l')
                    .long("lang")
                    .num_args(1)
                    .help("Edit with specific language"),
            )
            .arg(
                Arg::new("id")
                    .num_args(1)
                    .value_parser(clap::value_parser!(i32))
                    .help("question id"),
            )
            .arg(
                Arg::new("daily")
                    .short('d')
                    .long("daily")
                    .help("Edit today's daily challenge")
                    .action(ArgAction::SetTrue),
            )
            .group(
                ArgGroup::new("question-id")
                    .args(["id", "daily"])
                    .multiple(false)
                    .required(true),
            )
    }

    /// `edit` handler
    async fn handler(m: &ArgMatches) -> Result<()> {
        use crate::{Cache, cache::models::Question};
        use std::fs::File;
        use std::io::Write;
        use std::path::Path;

        let cache = Cache::new()?;

        let daily = m.get_one::<bool>("daily").unwrap_or(&false);
        let daily_id = if *daily {
            Some(cache.get_daily_problem_id().await?)
        } else {
            None
        };

        let id = m
            .get_one::<i32>("id")
            .copied()
            .or(daily_id)
            .ok_or(Error::NoneError)?;

        let problem = cache.get_problem(id)?;
        let mut conf = cache.to_owned().0.conf;

        let test_flag = conf.code.test;

        let p_desc_comment = problem.desc_comment(&conf);
        // condition language
        if m.contains_id("lang") {
            conf.code.lang = m
                .get_one::<String>("lang")
                .ok_or(Error::NoneError)?
                .to_string();
            conf.sync()?;
        }

        let lang = &conf.code.lang;
        let path = crate::helper::code_path(&problem, Some(lang.to_owned()))?;

        if !Path::new(&path).exists() {
            let mut qr = serde_json::from_str(&problem.desc);
            if qr.is_err() {
                qr = Ok(cache.get_question(id).await?);
            }

            let question: Question = qr?;

            let mut file_code = File::create(&path)?;
            let question_desc = question.desc_comment(&conf) + "\n";

            let test_path = crate::helper::test_cases_path(&problem)?;

            let mut flag = false;
            for d in question.defs.0 {
                if d.value == *lang {
                    flag = true;
                    if conf.code.comment_problem_desc {
                        file_code.write_all(p_desc_comment.as_bytes())?;
                        file_code.write_all(question_desc.as_bytes())?;
                    }
                    if let Some(inject_before) = &conf.code.inject_before {
                        for line in inject_before {
                            file_code.write_all((line.to_string() + "\n").as_bytes())?;
                        }
                    }
                    if conf.code.edit_code_marker {
                        file_code.write_all(
                            (conf.code.comment_leading.clone()
                                + " "
                                + &conf.code.start_marker
                                + "\n")
                                .as_bytes(),
                        )?;
                    }
                    file_code.write_all((d.code.to_string() + "\n").as_bytes())?;
                    if conf.code.edit_code_marker {
                        file_code.write_all(
                            (conf.code.comment_leading.clone()
                                + " "
                                + &conf.code.end_marker
                                + "\n")
                                .as_bytes(),
                        )?;
                    }
                    if let Some(inject_after) = &conf.code.inject_after {
                        for line in inject_after {
                            file_code.write_all((line.to_string() + "\n").as_bytes())?;
                        }
                    }

                    if test_flag {
                        let mut file_tests = File::create(&test_path)?;
                        file_tests.write_all(question.all_cases.as_bytes())?;
                    }
                }
            }

            // if language is not found in the list of supported languges clean up files
            if !flag {
                std::fs::remove_file(&path)?;

                if test_flag {
                    std::fs::remove_file(&test_path)?;
                }

                return Err(
                    anyhow!("This question doesn't support {lang}, please try another").into(),
                );
            }
        }

        // Get arguments of the editor
        //
        // for example:
        //
        // ```toml
        // [code]
        // editor = "emacsclient"
        // editor_args = [ "-n", "-s", "doom" ]
        // ```
        //
        // ```rust
        // Command::new("emacsclient").args(&[ "-n", "-s", "doom", "<problem>" ])
        // ```
        let mut args: Vec<String> = Default::default();
        if let Some(editor_args) = conf.code.editor_args {
            args.extend_from_slice(&editor_args);
        }

        // Set environment variables for editor
        //
        // for example:
        //
        // ```toml
        // [code]
        // editor = "nvim"
        // editor_envs = [ "XDG_DATA_HOME=...", "XDG_CONFIG_HOME=...", "XDG_STATE_HOME=..." ]
        // ```
        //
        // ```rust
        // Command::new("nvim").envs(&[ ("XDG_DATA_HOME", "..."), ("XDG_CONFIG_HOME", "..."), ("XDG_STATE_HOME", "..."), ]);
        // ```
        let mut envs: HashMap<String, String> = Default::default();
        if let Some(editor_envs) = &conf.code.editor_envs {
            for env in editor_envs.iter() {
                let parts: Vec<&str> = env.split('=').collect();
                if parts.len() == 2 {
                    let name = parts[0].trim();
                    let value = parts[1].trim();
                    envs.insert(name.to_string(), value.to_string());
                } else {
                    return Err(anyhow!("Invalid editor environment variable: {env}").into());
                }
            }
        }

        args.push(path);
        std::process::Command::new(conf.code.editor)
            .envs(envs)
            .args(args)
            .status()?;

        // Update file status since we just created/edited the file
        cache.update_problem_file_status(problem.id, true)?;

        Ok(())
    }
}
