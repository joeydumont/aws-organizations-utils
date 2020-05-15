use rusoto_credential::ProvideAwsCredentials;
use rusoto_s3::{S3Client, S3};
use rusoto_sts::{self, StsAssumeRoleSessionCredentialsProvider, StsClient};
use std::io::{self, Write};
use std::process::Command;
use serde::{Deserialize,Serialize};
use serde_json::{Result, Value};

#[derive(Serialize, Deserialize)]
struct Resources {
    AccountId: String,
    Resources: serde_json::Value,
}

// Architecture: Call assume_role() on
// StsAssumeRoleSessionCredentialsProvider to get the credentials of the
// assumed role. Write these credentials to a file to be used as the
// AWS_SHARED_CREDENTIALS_FILE. Repeat for all accounts in the organization.
// For each section of that file, forward the command that was given to the
// program to the AWS CLI with the different profiles. Collate the results
// in a single JSON response.
pub async fn list_resources(
    sts: &StsClient,
    role_name: &str,
    account_ids: Vec<String>,
    command_string: &str,
) {
    let mut resources = Vec::new();
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

        let creds = rusoto_credential::AutoRefreshingProvider::new(provider)
            .unwrap()
            .credentials()
            .await
            .unwrap();

        let output = Command::new("aws")
            .env("AWS_ACCESS_KEY_ID", creds.aws_access_key_id())
            .env("AWS_SECRET_ACCESS_KEY", creds.aws_secret_access_key())
            .env("AWS_SESSION_TOKEN", creds.token().as_ref().unwrap())
            .args(command_string.split(" "))
            .output()
            .expect("failed to execute process");

        // Error handling.
        //io::stdout().write_all(&output.stdout).unwrap();
        //io::stderr().write_all(&output.stderr).unwrap();

        // Add to our base JSON.
        resources.push(Resources {
            AccountId: account,
            Resources: serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap(),
        })
    }

    let j = serde_json::to_string_pretty(&resources).unwrap();
    println!("{}", j);
}
