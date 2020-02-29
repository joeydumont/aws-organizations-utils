use rusoto_core;
use rusoto_organizations::{OrganizationsClient};
use env_logger;

mod list_accounts;


fn main() {

    let _ = env_logger::try_init();

    let client = OrganizationsClient::new(
        rusoto_core::Region::UsEast1
    );

    let table = list_accounts::list_accounts_per_ou(&client);

    println!("{}", table);

}
