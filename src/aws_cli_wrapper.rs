use rusoto_s3::{S3Client, S3};
use rusoto_sts::{StsAssumeRoleSessionCredentialsProvider, StsClient};
use rusoto_organizations::OrganizationsClient;

use crate::list_accounts;

// Architecture: Call assume_role() on
// StsAssumeRoleSessionCredentialsProvider to get the credentials of the
// assumed role. Write these credentials to a file to be used as the
// AWS_SHARED_CREDENTIALS_FILE. Repeat for all accounts in the organization.
// For each section of that file, forward the command that was given to the
// program to the AWS CLi with the different profiles. Collate the results
// in a single JSON response.

pub fn list_buckets(sts: &StsClient, role_name: &str) {
    // List buckets in other accounts using STS.
    // Needs credentials from the master account.
    let client = OrganizationsClient::new(rusoto_core::Region::UsEast1);
    let accounts = list_accounts::list_accounts(&client);

    for (account, _) in accounts {
        let provider = StsAssumeRoleSessionCredentialsProvider::new(
            sts.clone(),
            format!("arn:aws:iam::{}:role/{}",account.id.unwrap(), role_name).to_owned(),
            "default".to_owned(),
            None,None,None,None
        );

        let auto_refreshing_provider = rusoto_credential::AutoRefreshingProvider::new(provider).unwrap();

        let s3_client = S3Client::new_with(
            rusoto_core::HttpClient::new().unwrap(),
            auto_refreshing_provider,
            rusoto_core::Region::CaCentral1
        );

        let list_of_buckets = s3_client.list_buckets().sync().unwrap();

        println!("{:#?}", list_of_buckets);
    }
}
