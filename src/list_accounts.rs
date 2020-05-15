use rusoto_organizations::{
    Account, DescribeAccountRequest, DescribeOrganizationalUnitRequest, ListChildrenRequest,
    ListRootsRequest, OrganizationalUnit, Organizations, OrganizationsClient, Root,
};
use std::{thread, time};
use async_recursion::async_recursion;

/// In an AWS Organizations, each node of the account tree can either be an OU or an account.
#[derive(Debug)]
enum OrgNode {
    Ou(OrganizationalUnit),
    Account(Account),
}

/// m-ary tree to represent the AWS organization.
#[derive(Debug)]
struct OrgTree {
    root: OrgNode,
    children: Option<Vec<Box<OrgTree>>>,
}

impl OrgTree {
    pub fn new(root: OrgNode) -> OrgTree {
        OrgTree {
            root: root,
            children: None,
        }
    }

    /// Visits OUs in a preorder fashiong, listing all accounts at each level.
    pub fn iterative_preorder_ou(&self) -> Vec<&OrgTree> {
        let mut stack: Vec<&OrgTree> = Vec::new();
        let mut res: Vec<&OrgTree> = Vec::new();

        stack.push(self);
        while !stack.is_empty() {
            let node = stack.pop().unwrap();
            res.push(node);

            if let Some(ref children) = node.children {
                for elem in children {
                    stack.push(elem);
                }
            }

            println!("Stack:");
            for elem in stack.iter() {
                println!("{:#?}", elem.root);
            }
        }
        res
    }
}

/// Starting from a root of the AWS Organization, we first request all of its child OUs, and then
/// all of its child accounts. This ensures that preorder traversal list all accounts that belong
/// to an OU before listing accounts that belong to nested OUs.
///
/// The `thread::sleep` in the loops are necessary to not hit AWS Organizations' API limits.
///
/// # Example
///
/// Say we had the following tree:
/// ```ascii
///
///                            R
///                           / \
///                          /   \
///                         OU    A
///                        / | \
///                       /  |  \
///                      /   |   \
///                     OU  A1    A2
/// ```
/// Preorder traversal would print nodes in this order:
///  * R:A
///  * R:OU:A2
///  * R:OU:A1
#[async_recursion]
async fn recursively_build_account_tree(client: &OrganizationsClient, node: &mut OrgTree) {
    match &node.root {
        OrgNode::Ou(v) => {
            let list_children_request = ListChildrenRequest {
                parent_id: v.id.as_ref().unwrap().to_string(),
                child_type: "ORGANIZATIONAL_UNIT".to_string(),
                max_results: None,
                next_token: None,
            };

            let list_children_response = client
                .list_children(list_children_request)
                .await
                .unwrap()
                .children
                .unwrap();

            if list_children_response.len() > 0 {
                node.children = Some(Vec::new());

                for element in list_children_response.iter() {
                    // Describe the OU.
                    let describe_org_unit_request = DescribeOrganizationalUnitRequest {
                        organizational_unit_id: element.id.as_ref().unwrap().to_string(),
                    };

                    let describe_org_unit_response = client
                        .describe_organizational_unit(describe_org_unit_request)
                        .await
                        .unwrap()
                        .organizational_unit
                        .unwrap();
                    if let Some(v) = &mut node.children {
                        v.push(Box::new(OrgTree::new(OrgNode::Ou(
                            describe_org_unit_response,
                        ))));

                        thread::sleep(time::Duration::from_millis(200));
                    }
                }
            }

            // Request accounts in this OU.
            let list_children_request = ListChildrenRequest {
                parent_id: v.id.as_ref().unwrap().to_string(),
                child_type: "ACCOUNT".to_string(),
                max_results: None,
                next_token: None,
            };

            let list_children_response = client
                .list_children(list_children_request)
                .await
                .unwrap()
                .children
                .unwrap();

            if list_children_response.len() > 0 {
                match &mut node.children {
                    Some(_) => (),
                    None => {
                        node.children = Some(Vec::new());
                    }
                }

                for element in list_children_response {
                    let describe_account_request = DescribeAccountRequest {
                        account_id: element.id.unwrap(),
                    };

                    let describe_account_response = client
                        .describe_account(describe_account_request)
                        .await
                        .unwrap()
                        .account
                        .unwrap();

                    node.children
                        .as_mut()
                        .unwrap()
                        .push(Box::new(OrgTree::new(OrgNode::Account(
                            describe_account_response,
                        ))));

                    thread::sleep(time::Duration::from_millis(200));
                }
            }

            // Recursively build the tree.
            match &mut node.children {
                Some(v) => {
                    for element in v {
                        recursively_build_account_tree(client, &mut *element).await;
                    }
                }

                None => (),
            };
        }

        OrgNode::Account(_) => (),
    }
}

fn build_ou_prefix(ou_prefix_vec: &Vec<String>) -> String {
    if ou_prefix_vec.len() > 1 {
        ou_prefix_vec[1..].join(":")
    } else {
        "".to_string()
    }
}

/// Fetches all of the accounts in the AWS Organizations and outputs
/// them in a Markdown-compatible table.
pub async fn list_accounts(client: &OrganizationsClient) -> Vec<(Account, String)> {
    let root_request = ListRootsRequest {
        max_results: None,
        next_token: None,
    };

    let root: Root = client
        .list_roots(root_request)
        .await
        .unwrap()
        .roots
        .unwrap()[0]
        .clone();

    // Coerce the root into an OU (kinda cheating).
    let root_as_ou = OrganizationalUnit {
        arn: root.arn,
        id: root.id,
        name: root.name,
    };

    let mut org_tree = OrgTree::new(OrgNode::Ou(root_as_ou));

    // Recursively build the account tree.
    match &org_tree.root {
        OrgNode::Ou(_) => {
            recursively_build_account_tree(&client, &mut org_tree).await;
        }

        _ => panic!(
            "The root node should be an OU, no whathever it is: {:?}",
            org_tree
        ),
    }

    // When printing the accounts, I print the account's parent OUs' names.
    // To do that, I need to keep of how deep I am in the tree struture, and
    // what the parent OUs are.
    //
    // I use the a typical stack-based preorder traversal algorithm.
    // To keep track of the OUs,, I define what I can an height stack that is vec that stores
    // the degree of the node that we pop from the stack. Each time we push an OU, we push
    // degree of that OU to the height stack. When we pop any node, we decrement the
    // degree counter in the vec until it reaches 0, then we pop the stack. We pop
    // until there are no non-zero counters.
    let mut accounts: Vec<(Account, String)> = Vec::new();
    let mut stack: Vec<&OrgTree> = Vec::new();
    let mut height_stack: Vec<usize> = Vec::new();
    let mut ou_prefix: Vec<String> = Vec::new();

    stack.push(&org_tree);
    height_stack.push(1);
    while !stack.is_empty() {
        let node = stack.pop().unwrap();

        if let Some(last) = height_stack.last_mut() {
            *last -= 1;
        }

        match &node.root {
            OrgNode::Ou(ou) => {
                if let Some(ref children) = node.children {
                    ou_prefix.push(ou.name.as_ref().unwrap().to_string());
                    height_stack.push(children.len());
                    for elem in children {
                        stack.push(elem);
                    }
                }
            }
            OrgNode::Account(account) => {
                accounts.push((account.clone(), build_ou_prefix(&ou_prefix)));
            }
        }

        while let Some(0) = height_stack.last() {
            height_stack.pop();
            ou_prefix.pop();
        }
    }

    accounts
}
