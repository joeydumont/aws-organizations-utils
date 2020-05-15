use clap::{load_yaml, App, Arg, SubCommand};
use comfy_table::{Attribute, Cell, Row, Table};
use env_logger;
use rusoto_core;
use rusoto_organizations::OrganizationsClient;
use rusoto_sts::StsClient;
use tokio;

mod aws_cli_wrapper;
mod list_accounts;

#[tokio::main]
async fn main() {
    // Parse CLI arguments.
    let _ = env_logger::try_init();
    //let yaml = load_yaml!("cli.yaml");
    //let matches = App::from(yaml).get_matches();

    let matches = App::new("aws-organization-utils")
        .author("Joey Dumont, joey.dumont@gmail.com")
        .version("0.1.0")
        .about("Wrapper around the AWS CLI to query resources accross an AWS Organization")
        .subcommand(
            SubCommand::with_name("list-accounts").about("List all accounts in the organization"),
        )
        .subcommand(
            SubCommand::with_name("list-resources").about("List resources accross the organization.")
            .arg(Arg::with_name("role-name")
                .help("Name of the role to assume in each account.")
                .required(true)
                .takes_value(true)
                .long("role-name"))
            .arg(Arg::with_name("account-ids")
                .help("(Optional) Comma separated list of accounts to query. If not present, all accounts in the organizatino will be queried.")
                .required(false)
                .long("account-ids")
                .takes_value(true))
            .arg(Arg::with_name("exclude-ous")
                .help("(Optional) Comma separated list of OUs to exclude.")
                .required(false)
                .long("exclude-ous")
                .takes_value(true)
                )
            .arg(Arg::with_name("aws-cli-command").last(true).multiple(true)))
        .get_matches();

    match matches.subcommand() {
        ("list-accounts", Some(_)) => {
            // Describe organization.
            let client = OrganizationsClient::new(rusoto_core::Region::UsEast1);
            let vec = list_accounts::list_accounts(&client).await;
            let mut table = Table::new();
            const MARKDOWN: &str = "||  |-|||           ";
            table.load_preset(MARKDOWN).set_header(vec![
                Cell::new("Account name").add_attribute(Attribute::Bold),
                Cell::new("Account ID").add_attribute(Attribute::Bold),
                Cell::new("OU name").add_attribute(Attribute::Bold),
                Cell::new("Email").add_attribute(Attribute::Bold),
            ]);
            for element in vec {
                table.add_row(Row::from(vec![
                    element.0.name.as_ref().unwrap().to_string(),
                    element.0.id.as_ref().unwrap().to_string(),
                    element.1,
                    element.0.email.as_ref().unwrap().to_string(),
                ]));
            }
            println!("{}", table);
        }

        ("list-resources", Some(subcmd)) => {
            let sts = StsClient::new(rusoto_core::Region::UsEast1);
            let role_name = subcmd.value_of("role-name").unwrap();
            let command_string = subcmd
                .values_of("aws-cli-command")
                .unwrap()
                .collect::<Vec<_>>()
                .join(" ");
            let mut account_ids: Vec<String> = Vec::new();
            if let Some(account_ids_input) = subcmd.value_of("account-ids") {
                for account in account_ids_input.split(",") {
                    account_ids.push(account.to_string());
                }
            } else {
                // List buckets in other accounts using STS.
                // Needs credentials from the master account.
                let client = OrganizationsClient::new(rusoto_core::Region::UsEast1);
                let temp_account_ids = list_accounts::list_accounts(&client).await;

                let mut excluded_ous = Vec::new();
                if let Some(excluded_ous_input) = subcmd.value_of("exclude-ous") {
                    for ou in excluded_ous_input.split(",") {
                        excluded_ous.push(ou.to_string());
                    }
                }

                for (account, ou) in temp_account_ids {
                    if !excluded_ous.contains(&ou) {
                        account_ids.push(account.id.unwrap());
                    }
                }
            }

            aws_cli_wrapper::list_resources(&sts, role_name, account_ids, &command_string).await;
        }

        _ => (),
    }

}
