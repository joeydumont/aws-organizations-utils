use rusoto_organizations::{
    Organizations,
    OrganizationsClient,
    ListRootsRequest,

    ListRootsResponse,
    ListChildrenRequest,
    ListChildrenResponse,
};

use comfy_table::{Table,Cell,ContentArrangement,Attribute};

#[derive(Debug)]
enum OrgNode {
    Ou(String),
    Account(String),
}

#[derive(Debug)]
struct OrgTree {
    root: OrgNode,
    children: Option<Vec<Box<OrgTree>>>,
}

impl OrgTree {
    fn new(root: OrgNode) -> OrgTree {
        OrgTree {
            root: root,
            children: None,
        }
    }
}

fn recursively_build_account_tree(client: &OrganizationsClient, node: &mut OrgTree) {
    match &node.root {
        OrgNode::Ou(v) => {
            let list_children_request = ListChildrenRequest {
                parent_id: v.to_string(),
                child_type: "ORGANIZATIONAL_UNIT".to_string(),
                max_results: None,
                next_token: None,
            };

            let list_children_response = client.list_children(list_children_request).sync().unwrap().children.unwrap();

            node.children = Some(Vec::new());

            for element in list_children_response {
                if let Some(v) = &mut node.children {
                    v.push(Box::new(OrgTree::new(OrgNode::Ou(element.id.unwrap()))));
                }
            }

            // Request accounts in this OU.
            let list_children_request = ListChildrenRequest {
                parent_id: v.to_string(),
                child_type: "ACCOUNT".to_string(),
                max_results: None,
                next_token: None,
            };

            let list_children_response = client.list_children(list_children_request).sync().unwrap().children.unwrap();

            match &mut node.children {
                Some(v) => {
                    for element in list_children_response {
                        v.push(Box::new(OrgTree::new(OrgNode::Account(element.id.unwrap()))));
                    }
                },

                None => (),
            }

            // Recursively build the tree.
            match &mut node.children {
                Some(v) => {
                    for element in v {
                        recursively_build_account_tree(client, &mut *element);
                    }
                },

                None => (),
            }
        }

        OrgNode::Account(_) => ()
    }
}

fn traverse_the_tree(node: &OrgTree, ops: )

pub fn list_accounts_per_ou(client: &OrganizationsClient) -> Table {

    let root_request = ListRootsRequest {
        max_results: None,
        next_token: None,
    };

    let mut org_tree = OrgTree::new(OrgNode::Ou(client.list_roots(root_request).sync().unwrap().roots.unwrap()[0].id.clone().unwrap()));


    // Recursively build the account tree.
    match &org_tree.root {
        OrgNode::Ou(_) => {
            recursively_build_account_tree(&client, &mut org_tree);
        },

        _ => {
            panic!("The root node should be an OU, no whathever it is: {:?}", org_tree)
        }
    }

    println!("{:#?}", org_tree);
    let mut table = Table::new();
    table.set_header(
        vec![
            Cell::new("Account name").add_attribute(Attribute::Bold),
            Cell::new("Account ID").add_attribute(Attribute::Bold),
            Cell::new("OU name").add_attribute(Attribute::Bold),
            Cell::new("Email").add_attribute(Attribute::Bold),
        ]
    );

    ///

    table
}