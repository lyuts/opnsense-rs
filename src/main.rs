mod model;

use anyhow::bail;
use anyhow::ensure;
use anyhow::Context;
use clap::Arg;
use clap::ArgAction;
use clap::Command;
use model::{get_apis, Api};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::debug;
use tracing::trace;
use tracing::Level;

const CONFIG_FILE: &str = ".opn.cfg";

#[derive(Deserialize)]
#[serde(untagged)]
enum AddApiKeyResponse {
    Success {
        result: String,
        #[allow(dead_code)]
        hostname: String,
        key: String,
        secret: String,
    },
    Failure {
        status: u16,
        message: String,
    },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SearchApiKeyResponse {
    #[serde(rename_all = "camelCase")]
    Success {
        #[allow(dead_code)]
        total: u16,
        #[allow(dead_code)]
        row_count: u16,
        #[allow(dead_code)]
        current: u16,
        rows: Vec<ApiKeyRow>,
    },
    Failure {
        #[allow(dead_code)]
        status: u16,
        #[allow(dead_code)]
        message: String,
    },
}

#[derive(Debug, Deserialize)]
struct ApiKeyRow {
    #[allow(dead_code)]
    username: String,
    key: String,
    id: String,
}

fn call(
    endpoint: &str,
    api: &Api,
    params: Vec<String>,
    insecure: bool,
    key: &str,
    secret: &str,
) -> anyhow::Result<String> {
    let method = reqwest::Method::from_bytes(api.method.to_uppercase().as_bytes())?;
    let mut url = format!(
        "{}/api/{}/{}/{}",
        endpoint, api.module, api.controller, api.command
    );

    for p in &params {
        url.push_str(&format!("/{p}"));
    }

    debug!("api: {:?}", api);
    debug!("url: {}", url);
    let has_required_params = api.parameters.iter().any(|p| p.1);
    let resp = if !has_required_params && method == reqwest::Method::GET {
        debug!("sending request without body");
        reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(insecure)
            .build()?
            .request(method, url)
            .basic_auth(key, Some(secret))
            .send()?
            .text()?
    } else {
        debug!("sending request with body");
        reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(insecure)
            .build()?
            .request(method, url)
            .basic_auth(key, Some(secret))
            .json(&serde_json::json!({}))
            .send()?
            .text()?
    };
    Ok(resp)
}

fn main() -> anyhow::Result<()> {
    let conf = ini::Ini::load_from_file(CONFIG_FILE)?;
    let profile_data = conf.section(Some("default")).context("Profile not found")?;
    let endpoint = profile_data
        .get("endpoint")
        .context("endpoint is missing in the profile.")?;
    let key = profile_data
        .get("key")
        .context("key is missing in the profile.")?;
    let secret = profile_data
        .get("secret")
        .context("secret is missing in the profile.")?;

    let apis: Vec<Api> = get_apis()?;

    let mut args = Command::new("opnsense cli")
        .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
        .arg(
            Arg::new("insecure")
                .short('k')
                .long("insecure")
                .required(false)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("login").about("Create API key and persist it for future commands."),
        )
        .subcommand(Command::new("logout").about("Destroy API key created by previous login."));

    let mut x: HashMap<String, HashMap<String, Vec<Api>>> = HashMap::new();
    for api in &apis {
        if !x.contains_key(&api.module) {
            x.insert(api.module.clone(), HashMap::new());
        }
        if !x.get(&api.module).unwrap().contains_key(&api.controller) {
            x.get_mut(&api.module)
                .unwrap()
                .insert(api.controller.clone(), Vec::new());
        }
        x.get_mut(&api.module)
            .unwrap()
            .get_mut(&api.controller)
            .unwrap()
            .push(api.clone());
    }

    for (module_name, controller_cmds) in x.iter() {
        let mut module_cmd = Command::new(module_name).display_order(0);
        for (controller_name, cmds) in controller_cmds.iter() {
            let mut controller_cmd = Command::new(controller_name).display_order(0);
            for api in cmds {
                let mut cmd = Command::new(api.command.clone())
                    .arg(Arg::new("body").short('b').long("body").default_value("{}"))
                    .display_order(0);
                for param in &api.parameters {
                    cmd = cmd.arg(
                        Arg::new(param.0.clone())
                            .long(param.0.clone())
                            .required(param.1),
                    )
                }
                controller_cmd = controller_cmd.subcommand(cmd);
            }
            module_cmd = module_cmd.subcommand(controller_cmd);
        }
        args = args.subcommand(module_cmd);
    }

    let matches = args.get_matches();
    let insecure = *matches.get_one::<bool>("insecure").unwrap_or(&false);
    let verbosity_level = match matches.get_count("verbose") {
        0 => Level::WARN,
        1 => Level::INFO,
        _ => Level::DEBUG,
    };

    tracing_subscriber::fmt()
        .with_file(true)
        .with_level(true)
        .with_line_number(true)
        .with_max_level(verbosity_level)
        .with_target(false)
        .with_thread_names(true)
        .init();

    let (module_name, module_cmd) = matches.subcommand().context("Must specify module.")?;
    debug!("Selected module: {}", module_name);
    match module_name {
        "login" => {
            let user = rpassword::prompt_password("User:")?;
            let pass = rpassword::prompt_password("Password:")?;
            trace!("[{}]", user);

            debug!("Logging in...");
            let response = call(
                endpoint,
                &Api {
                    method: "POST".to_owned(),
                    module: "auth".to_owned(),
                    controller: "user".to_owned(),
                    command: "addApiKey".to_owned(),
                    parameters: vec![("username".to_owned(), true)],
                },
                vec![user.to_owned()],
                insecure,
                &user,
                &pass,
            )?;

            trace!("Checking response...");

            match serde_json::from_str::<AddApiKeyResponse>(&response)? {
                AddApiKeyResponse::Success {
                    result,
                    hostname: _,
                    key,
                    secret,
                } => {
                    ensure!(result == "ok");
                    let mut updated_config = conf.clone();
                    updated_config
                        .section_mut(Some("default"))
                        .context("'default' section missing in config file.")?
                        .insert("key", key);
                    updated_config
                        .section_mut(Some("default"))
                        .context("'default' section missing in config file.")?
                        .insert("secret", secret);
                    updated_config.write_to_file(CONFIG_FILE)?;
                }
                AddApiKeyResponse::Failure { status, message } => bail!("[{}] {}", status, message),
            }
        }
        "logout" => {
            trace!("Discovering key id...");

            let response = call(
                endpoint,
                &Api {
                    method: "GET".to_owned(),
                    module: "auth".to_owned(),
                    controller: "user".to_owned(),
                    command: "searchApiKey".to_owned(),
                    parameters: vec![],
                },
                vec![],
                insecure,
                key,
                secret,
            )?;

            match serde_json::from_str::<SearchApiKeyResponse>(&response)
                .context("Failed to parse")?
            {
                SearchApiKeyResponse::Success {
                    total: _,
                    row_count: _,
                    current: _,
                    rows,
                } => {
                    let row = rows
                        .iter()
                        .find(|row| row.key == key)
                        .context("Key doesn't exist, nothing to delete.")?;
                    call(
                        endpoint,
                        &Api {
                            method: "POST".to_owned(),
                            module: "auth".to_owned(),
                            controller: "user".to_owned(),
                            command: "delApiKey".to_owned(),
                            parameters: vec![("id".to_owned(), true)],
                        },
                        vec![row.id.clone()],
                        insecure,
                        key,
                        secret,
                    )?;
                }
                _ => unimplemented!(),
            }
        }
        _ => {
            let (controller_name, controller_cmd) =
                module_cmd.subcommand().context("Must specify controller")?;
            debug!("Selected controller: {}", controller_name);
            let (command_name, command_cmd) = controller_cmd
                .subcommand()
                .context("Must specify command")?;
            debug!("Selected command: {}", command_name);

            let selected_api = apis
                .iter()
                .find(|a| {
                    a.module == module_name
                        && a.controller == controller_name
                        && a.command == command_name
                })
                .context("Unrecognized API.")?;

            let ordered_params: Vec<String> = selected_api
                .parameters
                .iter()
                .map(|(param_name, _is_required)| {
                    command_cmd
                        .get_one::<String>(param_name)
                        .unwrap_or(&String::new())
                        .to_owned()
                })
                .filter(|s| !s.is_empty())
                .collect();

            debug!("Selected command args: {:?}", ordered_params);

            let resp = call(
                endpoint,
                selected_api,
                ordered_params,
                insecure,
                key,
                secret,
            )?;
            println!("{resp}");
        }
    }

    Ok(())
}
