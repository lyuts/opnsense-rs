use nom::{
    character::complete::{alpha1, alphanumeric1, char, space0},
    combinator::{map, opt},
    multi::separated_list0,
    sequence::preceded,
    IResult, Parser,
};

#[derive(Clone, Debug)]
pub struct Api {
    pub method: String,
    pub module: String,
    pub controller: String,
    pub command: String,
    pub parameters: Vec<String>,
}

fn parameter(input: &str) -> IResult<&str, &str> {
    map(
        (
            char::<&str, nom::error::Error<&str>>('$'),
            alphanumeric1,
            opt(preceded(char('='), alphanumeric1)),
        ),
        |(_, p, _)| p.trim(),
    )
    .parse(input)
}

fn parameters(input: &str) -> IResult<&str, Vec<&str>> {
    separated_list0(char(','), parameter).parse(input)
}

fn api_model(input: &str) -> IResult<&str, Api> {
    map(
        (
            alpha1,
            space0,
            alpha1,
            space0,
            alpha1,
            space0,
            alpha1,
            opt(space0),
            opt(parameters),
        ),
        |(method, _, module, _, controller, _, command, _, parameters)| Api {
            method: method.to_owned(),
            module: module.to_owned(),
            controller: controller.to_owned(),
            command: command.to_owned(),
            parameters: parameters
                .unwrap_or_default()
                .iter()
                .map(ToString::to_string)
                .collect(),
        },
    )
    .parse(input)
}

pub fn get_apis() -> anyhow::Result<Vec<Api>> {
    let model_content = include_str!("model.txt");

    Ok(model_content
        .split('\n')
        .filter(|l| !l.trim().is_empty())
        .map(|l| api_model(l).inspect_err(|e| eprintln!("Failed to parse line: {}", e)))
        .map(|r| r.unwrap().1)
        .collect::<Vec<Api>>())
}
