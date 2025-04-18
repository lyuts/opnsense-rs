mod model;

use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Context;
use clap::arg;
use clap::Arg;
use clap::ArgAction;
use clap::Command;
use clap::{Args, Parser as ClapParser, Subcommand};
use model::{get_apis, Api};
use serde::Deserialize;
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let _apis: Vec<Api> = get_apis()?;

    let conf = ini::Ini::load_from_file(".opn.ini")?;
    let profile_data = conf
        .section(Some("default"))
        .ok_or(anyhow!("Profile not found."))?;
    let key = profile_data
        .get("key")
        .ok_or(anyhow!("key is missing in the profile."))?;
    let secret = profile_data
        .get("secret")
        .ok_or(anyhow!("secret is missing in the profile."))?;

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
            x.get_mut(&api.module).unwrap().insert(api.controller.clone(), Vec::new());
        }
        x.get_mut(&api.module).unwrap().get_mut(&api.controller).unwrap().push(api.clone());
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

    println!("{:?}", matches.subcommand().unwrap());
    Ok(())
}
