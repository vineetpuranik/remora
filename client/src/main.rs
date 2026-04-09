use proto::policy::{policy_service_client::PolicyServiceClient, AccessRequest, TokenRequest};

#[tokio::main]
async fn main() {
    // construct the service client
    let mut client = PolicyServiceClient::connect("http://127.0.0.1:50051")
        .await
        .unwrap_or_else(|err| {
            eprintln!("ERROR: could not connect to control plane : {err}");
            std::process::exit(1);
        });

    println!("User: {}", "bob".to_string());

    // get the token for the user
    let token_response = client
        .issue_token(TokenRequest {
            user_id: "bob".to_string(),
        })
        .await
        .unwrap_or_else(|err| {
            eprintln!("ERROR: Issue token failed : {err}");
            std::process::exit(1);
        });

    let token = token_response.into_inner().token;
    println!("Token issued  : {token}");

    // check access based on in the issued token
    let access_response = client
        .check_access(AccessRequest {
            user_id: "alice".to_string(),
            resource: "api_internal".to_string(),
            action: "read".to_string(),
            token,
        })
        .await
        .unwrap_or_else(|err| {
            eprintln!("ERROR: Check Access failed : {err}");
            std::process::exit(1);
        });

    let response = access_response.into_inner();
    if response.allowed {
        println!("ALLOW - {}", response.reason);
    } else {
        println!("DENY - {}", response.reason);
    }

    return;
}
