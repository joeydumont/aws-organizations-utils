# aws-organizations-utils

This crate provides a few utilities to help manage AWS Organizations. Right now, it can:
  * fetch the full list of accounts and their OUs (assuming a single root);
  * provides a wrapper around the AWS CLI to query across multiple accounts

## Usage

To use this tool, you will need some access to the AWS Organization' master account. To build
the list of accounts, you need the following permissions:
```yaml
Version: '2012-10-17'
Statement:
  - Sid: AllowWalkAccountTree
    Effect: Allow
    Action:
      - organizations:DescribeOrganizationalUnit
      - organizations.DescribeAccount
      - organizations:ListRoots
      - organizations:ListChildren
    Resource: '*'
```

To build the list of accounts, use

```bash
aws-organizations-utils list-accounts
```

To query resources across accounts, this tool assumes that you have a named IAM role with the proper
permissions, and that you can assume that role from the master account. For instance, let's say
you have a role named `OrganizationViewOnlyRole` in each child account, and that role has a trust
a relationship with the master account. An example CloudFormation template would look like:

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Description: >-
  This role grants read-only access to all of the resources in an account.
Parameters:
  MasterAccountId:
    Type: String
    Description: Master account ID.
Resources:
  CrossAccountReadOnlyRole:
    Type: AWS::IAM::Role
    Properties:
      RoleName: OrganizationViewOnlyRole
      Description: >-
        Read-only to be assumed from the master account.
      Path: /itops/
      AssumeRolePolicyDocument:
        Version: '2012-10-17'
        Statement:
          - Effect: Allow
            Principal:
              AWS:
                - !Sub "arn:aws:iam::${MasterAccountId}:role/OrganizationWideViewOnly
            Action: "sts:AssumeRole"
      ManagedPolicyArns:
        - arn:aws:iam::aws:policy/ReadOnlyAccess
```

To list all the S3 buckets available in the organization, you would need credentials for the `OrganizationWideViewOnly` role, then run

```bash
aws-organizations-utils list-resources --role-name OrganizationWideViewOnly [ --account-ids <comma_separated_list_of_accounts> ] -- s3api list-buckets
```

If you don't provide a list of accounts IDs, it will attempt to build the account tree

This prints out the results in JSON format with the schema:
```json
[
    {
        "AccoundId": "000000000000",
        "Resources": api_response,
    },
    {
        ...
    }
]
```