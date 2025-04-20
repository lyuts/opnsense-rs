mod model;

use anyhow::Context;
use clap::Arg;
use clap::ArgAction;
use clap::Command;
use model::{get_apis, Api};
use std::collections::HashMap;

fn call(
    endpoint: &str,
    api: &Api,
    params: Vec<String>,
    insecure: bool,
    key: &str,
    secret: &str,
) -> anyhow::Result<()> {
    let method = reqwest::Method::from_bytes(api.method.to_uppercase().as_bytes())?;
    let mut url = format!(
        "{}/api/{}/{}/{}",
        endpoint, api.module, api.controller, api.command
    );

    for p in params {
        url.push_str(&format!("/{}", p));
    }

    println!("url: {}", url);
    let resp = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(insecure)
        .build()?
        .request(method, url)
        .basic_auth(key, Some(secret))
        // .json(&json!({}))
        .send()?
        .text()?;

    println!("==>>> {:?}", resp);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let conf = ini::Ini::load_from_file(".opn.ini")?;
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

    let mut args = Command::new("opnsense cli").arg(
        Arg::new("insecure")
            .short('k')
            .long("insecure")
            .required(false)
            .action(ArgAction::SetTrue),
    );

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
        let mut module_cmd = Command::new(module_name);
        for (controller_name, cmds) in controller_cmds.iter() {
            let mut controller_cmd = Command::new(controller_name);
            for api in cmds {
                let mut cmd = Command::new(api.command.clone());
                for param in &api.parameters {
                    cmd = cmd.arg(Arg::new(param).long(param))
                }
                controller_cmd = controller_cmd.subcommand(cmd);
            }
            module_cmd = module_cmd.subcommand(controller_cmd);
        }
        args = args.subcommand(module_cmd);
    }

    let matches = args.get_matches();
    let insecure = *matches.get_one::<bool>("insecure").unwrap_or(&false);

    let (module_name, module_cmd) = matches.subcommand().context("Must specify module.")?;
    println!("Selected module: {}", module_name);
    let (controller_name, controller_cmd) =
        module_cmd.subcommand().context("Must specify controller")?;
    println!("Selected controller: {}", controller_name);
    let (command_name, command_cmd) = controller_cmd
        .subcommand()
        .context("Must specify command")?;
    println!("Selected command: {}", command_name);

    let selected_api = apis
        .iter()
        .find(|a| {
            a.module == module_name && a.controller == controller_name && a.command == command_name
        })
        .context("Unrecognized API.")?;

    let ordered_params: Vec<String> = selected_api
        .parameters
        .iter()
        .map(|param_name| {
            command_cmd
                .get_one::<String>(param_name)
                .unwrap_or(&String::new())
                .to_owned()
        })
        .filter(|s| !s.is_empty())
        .collect();

    println!("Selected command args: {:?}", ordered_params);

    call(
        endpoint,
        selected_api,
        ordered_params,
        insecure,
        key,
        secret,
    )?;

    Ok(())
}
