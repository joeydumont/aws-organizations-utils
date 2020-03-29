use clap::{load_yaml, App};
use comfy_table::{Attribute, Cell, Row, Table};
use env_logger;
use rusoto_core;
use rusoto_organizations::OrganizationsClient;
use rusoto_sts::StsClient;

mod aws_cli_wrapper;
mod list_accounts;

fn main() {
    // Parse CLI arguments.
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from(yaml).get_matches();
    let _ = env_logger::try_init();

    match matches.subcommand() {
        ("list-accounts", Some(_)) => {
            // Describe organization.
            let client = OrganizationsClient::new(rusoto_core::Region::UsEast1);
            let vec = list_accounts::list_accounts(&client);
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
        ("list-buckets", Some(subcmd)) => {
            let sts = StsClient::new(rusoto_core::Region::UsEast1);
            let role_name = subcmd.value_of("role-name").unwrap();
            let mut account_ids : Vec<String> = Vec::new();
            if let Some(account_ids_input) = subcmd.value_of("account-ids") {
                for account in account_ids_input.split(",") {
                    account_ids.push(account.to_string());
                }
            } else {
                // List buckets in other accounts using STS.
                // Needs credentials from the master account.
                let client = OrganizationsClient::new(rusoto_core::Region::UsEast1);
                let temp_account_ids = list_accounts::list_accounts(&client);

                for (account, _) in temp_account_ids {
                    account_ids.push(account.id.unwrap());
                }
            }

            aws_cli_wrapper::list_buckets(&sts, role_name, account_ids);
        }
        _ => (),
    }
}
