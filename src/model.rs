use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, char, space1},
    combinator::{map, opt, recognize},
    multi::{many0_count, separated_list0},
    sequence::{pair, preceded},
    IResult, Parser,
};

#[derive(Clone, Debug)]
pub struct Api {
    pub method: String,
    pub module: String,
    pub controller: String,
    pub command: String,
    /// parameter + reguireq?
    pub parameters: Vec<(String, bool)>,
}

fn parameter(input: &str) -> IResult<&str, (&str, bool)> {
    map(
        (
            char::<&str, nom::error::Error<&str>>('$'),
            identifier,
            opt(preceded(char('='), alphanumeric1)),
        ),
        |(_, p, x)| (p.trim(), x.is_none()),
    )
    .parse(input)
}

fn parameters(input: &str) -> IResult<&str, Vec<(&str, bool)>> {
    separated_list0(char(','), parameter).parse(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))
    .parse(input)
}

fn api_model(input: &str) -> IResult<&str, Api> {
    map(
        (
            alt((alphanumeric1, tag("*"))),
            space1,
            identifier,
            space1,
            identifier,
            space1,
            identifier,
            opt(space1),
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
                .map(|(s, f)| (s.to_string(), *f))
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
        .map(|l| api_model(l).inspect_err(|e| eprintln!("Failed to parse line [{}]: {}", l, e)))
        .map(|r| r.unwrap().1)
        .collect::<Vec<Api>>())
}
