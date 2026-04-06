// Why using dashmap over Mutex<HashMap> ?
// Mutex<HashMap> serializes all reads. Only one thread can read at a time even though reads do not conflict.
// DashMap shards internally, so concurrent reads on different keys proceed in parallel.
// For a policy store thats read far more often than written, this matters.

// Send and Sync crash course.
// Send - a type is Send if its safe to transfer ownership to another thread.
// Arc<T> is Send if T is Send - you can move an Arc into tokio::spawn closure.
// Sync - a type is Sync if its safe to share a reference (&T) across multiple threads simultaneously.
// Arc<T> is Sync if T is Sync - multiple threads can hold a clone of the same Arc and dereference it concurrently.
// Send and Sync are what qualify a type to be shared via an Arc.
// They are usually derived automatically by the compiler based on the fields on the struct/ custom type.

// why Arc ??
// Arc : Atomic reference counting.
// Provides safe shared ownership across multiple threads.
// The count is increment / decremented with CPU level atomic instructions.
// This makes it safe to clone and share across threads.
// Each tokio::spawn task gets a clone of the Arc, pointing to the same underlying DashMap.
// No copying of data, just a bump of the counter.

// Arc provides shared ownership across multiple threads.
// Every thread holding an Arc clone has equal, shared , immutable access to the inner value.
// To mutate an Arc, we need to use additional synchronization primitives
// Mutex : Arc<Mutex<>> : one thread locks, mutates, and unlocks others wait.
// Read/Write lock : Arc<RwLock<T>>  - many reader or ONE writer at a time.

// Arc solves the lifetime problem. The inner types solve the mutation problem.

use dashmap::DashMap;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use proto::policy::{
    policy_service_server::{PolicyService, PolicyServiceServer},
    AccessRequest, AccessResponse, TokenRequest, TokenResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};

const JWT_SECRET: &[u8] = b"remora-dev-secret";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // subject = user_id
    pub exp: usize,  // expiry = unix timestamp
}

pub fn issue_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + 3600; // 1 hour from now

    let claims = Claims {
        sub: user_id.to_string(),
        exp,
    };

    encode(
        &Header::default(), // HS 256
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
}

pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(token_data.claims)
}

pub struct PolicyEngine {
    // user_id => list of resources allowed
    policies: Arc<DashMap<String, Vec<String>>>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let policies: Arc<DashMap<String, Vec<String>>> = Arc::new(DashMap::new());

        // seeding some policies at startup
        policies.insert(
            "alice".to_string(),
            vec!["db_internal".to_string(), "api_internal".to_string()],
        );
        policies.insert("bob".to_string(), vec!["api_internal".to_string()]);

        Self { policies }
    }

    pub fn check(&self, user_id: &str, resource: &str) -> bool {
        match self.policies.get(user_id) {
            Some(resources) => resources.contains(&resource.to_string()),
            None => false,
        }
    }
}

// Struct for gRPC service handler
pub struct ControlPlane {
    engine: PolicyEngine,
}

#[tonic::async_trait]
impl PolicyService for ControlPlane {
    async fn check_access(
        &self,
        request: Request<AccessRequest>,
    ) -> Result<Response<AccessResponse>, Status> {
        // why .into_inner()
        // request is of type tonic::Request<AccessRequest> : its a wrapper that tonic uses to carry the message plus metadata
        // into_inner() unwraps it and gives us just the AccessRequest - the actual protobuf message we care

        let req = request.into_inner();

        //validate jwt
        let claims = validate_token(&req.token)
            .map_err(|_| Status::unauthenticated("invalid or expired token"))?;

        // ensure token subject matches requested user
        if claims.sub != req.user_id {
            return Err(Status::permission_denied("Token subject policy mismatch"));
        }

        let allowed = self.engine.check(&req.user_id, &req.resource);

        Ok(Response::new(AccessResponse {
            allowed,
            reason: if allowed {
                "policy match".into()
            } else {
                "no policy match found".into()
            },
        }))
    }

    async fn issue_token(
        &self,
        request: Request<TokenRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        let req = request.into_inner();
        let token = issue_token(&req.user_id)
            .map_err(|err| Status::internal(format!("token generation failed:  {err}")))?;
        Ok(Response::new(TokenResponse { token }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse().unwrap_or_else(|err| {
        eprintln!("ERROR: Failed to parse addr with error : {err}");
        std::process::exit(1);
    });

    let control_plane = ControlPlane {
        engine: PolicyEngine::new(),
    };

    println!("Control plane listening on {}", addr);

    tonic::transport::Server::builder()
        .add_service(PolicyServiceServer::new(control_plane))
        .serve(addr)
        .await
        .unwrap_or_else(|err| {
            eprintln!("ERROR: Failed to start tonic server : {err}");
            std::process::exit(1);
        });

    Ok(())
}
