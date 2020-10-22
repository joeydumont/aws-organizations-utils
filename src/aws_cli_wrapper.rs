use rusoto_budgets::{Budgets, BudgetsClient, DescribeBudgetsRequest, DescribeBudgetsResponse};
use rusoto_credential::ProvideAwsCredentials;
use rusoto_sts::{self, StsAssumeRoleSessionCredentialsProvider, StsClient};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize)]
struct Resources {
    #[serde(rename(serialize = "AccountId"))]
    account_id: String,
    #[serde(rename(serialize = "Resources"))]
    resources: DescribeBudgetsResponse,
}

pub async fn list_budgets(sts: &StsClient, role_name: &str, account_ids: Vec<String>) {
    let mut budgets = Vec::new();
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

        let budgets_client = BudgetsClient::new_with(
            rusoto_core::HttpClient::new().unwrap(),
            auto_refreshing_provider,
            rusoto_core::Region::Custom {
                name: "us-east-1".to_owned(),
                endpoint: "https://budgets.amazonaws.com".to_owned(),
            },
        );

        let budget_request = DescribeBudgetsRequest {
            account_id: account.clone(),
            ..DescribeBudgetsRequest::default()
        };

        let list_of_budgets = budgets_client
            .describe_budgets(budget_request)
            .await
            .unwrap();

        println!("{:#?}", list_of_budgets);

        // Add to our base JSON.
        budgets.push(Resources {
            account_id: account,
            resources: list_of_budgets,
        })
    }

    let j = serde_json::to_string_pretty(&budgets).unwrap();
    println!("{}", j);
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
            .env("AWS_ACCOUNT_ID", account.clone())
            .args(command_string.split(' '))
            .output()
            .expect("failed to execute process");

        // Add to our base JSON.
        resources.push(Resources {
            account_id: account,
            resources: serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap(),
        })
    }

    let j = serde_json::to_string_pretty(&resources).unwrap();
    println!("{}", j);
}
