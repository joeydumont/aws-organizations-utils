use rusoto_s3::{S3Client, S3};
use rusoto_sts::{StsAssumeRoleSessionCredentialsProvider, StsClient};

// Architecture: Call assume_role() on
// StsAssumeRoleSessionCredentialsProvider to get the credentials of the
// assumed role. Write these credentials to a file to be used as the
// AWS_SHARED_CREDENTIALS_FILE. Repeat for all accounts in the organization.
// For each section of that file, forward the command that was given to the
// program to the AWS CLi with the different profiles. Collate the results
// in a single JSON response.

pub fn list_buckets(sts: &StsClient, role_name: &str, account_ids: Vec<String>) {
    for account in account_ids {
        let provider = StsAssumeRoleSessionCredentialsProvider::new(
            sts.clone(),
            format!("arn:aws:iam::{}:role/{}", account, role_name).to_owned(),
            "default".to_owned(),
            None,
            None,
            None,
            None,
        );

        let auto_refreshing_provider =
            rusoto_credential::AutoRefreshingProvider::new(provider).unwrap();

        let s3_client = S3Client::new_with(
            rusoto_core::HttpClient::new().unwrap(),
            auto_refreshing_provider,
            rusoto_core::Region::CaCentral1,
        );

        let list_of_buckets = s3_client.list_buckets().sync().unwrap();

        println!("{:#?}", list_of_buckets);
    }
}
