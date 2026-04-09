use crate::PassClient;
pub use crate::PlanType;
use crate::common::{CodeResponse, SUCCESS_CODE};
use crate::test_tools::client_features::TestClientFeatures;
use crate::test_tools::{init_session, setup_user_access};
pub use muon::http::Method;
use muon_test::server::{ProtonAPI, Request, Response};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub type TestPassClient = PassClient<muon_test::server::TestContext<()>>;

pub trait MuonServerExt {
    fn handler<P, F>(&self, path: P, handler: F) -> Arc<AtomicBool>
    where
        P: AsRef<str> + Send + Sync + 'static,
        F: Fn(&Request) -> Option<Response> + Send + Sync + 'static;
    fn handler_with_method<P, F>(&self, method: Method, path: P, handler: F) -> Arc<AtomicBool>
    where
        P: AsRef<str> + Send + Sync + 'static,
        F: Fn(&Request) -> Option<Response> + Send + Sync + 'static;
}

impl MuonServerExt for ProtonAPI {
    fn handler<P, F>(&self, path: P, handler: F) -> Arc<AtomicBool>
    where
        P: AsRef<str> + Send + Sync + 'static,
        F: Fn(&Request) -> Option<Response> + Send + Sync + 'static,
    {
        let hit = Arc::new(AtomicBool::new(false));
        let hit_clone = hit.clone();
        self.add_handler(move |req| {
            if req.uri().path().eq(path.as_ref()) {
                let res = handler(req);
                if res.is_some() {
                    hit_clone.store(true, Ordering::Relaxed);
                }

                res
            } else {
                None
            }
        });

        hit
    }

    fn handler_with_method<P, F>(&self, method: Method, path: P, handler: F) -> Arc<AtomicBool>
    where
        P: AsRef<str> + Send + Sync + 'static,
        F: Fn(&Request) -> Option<Response> + Send + Sync + 'static,
    {
        let hit = Arc::new(AtomicBool::new(false));
        let hit_clone = hit.clone();
        self.add_handler(move |req| {
            let req_method_name = req.method().as_str().to_lowercase();
            let expected_method_name = method.to_string().to_lowercase();
            if req.uri().path().eq(path.as_ref()) && req_method_name == expected_method_name {
                let res = handler(req);
                if res.is_some() {
                    hit_clone.store(true, Ordering::Relaxed);
                }

                res
            } else {
                None
            }
        });

        hit
    }
}

// Create a TestPassClient from an already-created TestClient and ProtonAPI (without user data setup)
pub async fn make_test_pass_client(
    raw_client: muon_test::server::TestClient<()>,
    api: &ProtonAPI,
) -> TestPassClient {
    let key = pass_domain::crypto::generate_encryption_key();
    let session = raw_client
        .new_session_without_credentials(())
        .await
        .expect("Error creating session");
    init_session(api, session).await;
    TestPassClient::new(
        raw_client,
        Arc::new(TestClientFeatures::new(key)),
        pass_domain::AccountType::User,
    )
}

// Create a TestPassClient for a personal access token session (without user data setup)
pub async fn make_test_pass_pat_client(
    raw_client: muon_test::server::TestClient<()>,
    api: &ProtonAPI,
) -> TestPassClient {
    let key = pass_domain::crypto::generate_encryption_key();
    let session = raw_client
        .new_session_without_credentials(())
        .await
        .expect("Error creating session");
    init_session(api, session).await;
    TestPassClient::new(
        raw_client,
        Arc::new(TestClientFeatures::new(key)),
        pass_domain::AccountType::PersonalAccessToken,
    )
}

// Create a TestPassClient with full user data setup (addresses, keys, salts)
pub async fn make_test_pass_client_with_setup(
    raw_client: muon_test::server::TestClient<()>,
    api: &ProtonAPI,
    plan: PlanType,
) -> TestPassClient {
    super::setup_user_data::setup(api);
    let client = make_test_pass_client(raw_client, api).await;
    client
        .setup_key_passphrases(super::setup_user_data::TEST_PASSPHRASE)
        .await
        .expect("Error setting up passphrases");
    setup_user_access(api, plan);
    client
}

#[derive(serde::Serialize)]
pub struct Empty;

pub fn success<R: serde::Serialize>(res: R) -> Option<Response> {
    let body = serde_json::to_vec(&res).unwrap();
    Some(
        Response::builder()
            .status(200)
            .body(axum::body::Body::from(body))
            .unwrap(),
    )
}

pub fn success_code() -> Option<Response> {
    let body = serde_json::to_vec(&CodeResponse { code: SUCCESS_CODE }).unwrap();
    Some(
        Response::builder()
            .status(200)
            .body(axum::body::Body::from(body))
            .unwrap(),
    )
}

#[macro_export]
macro_rules! last_request {
    ($recorder:expr) => {{
        let requests = $recorder.read();
        let req = requests
            .into_iter()
            .last()
            .expect("Failed to get last request");

        let bytes = req.body().to_vec();
        serde_json::from_slice(&bytes).expect("Failed to parse request")
    }};
}

#[macro_export]
macro_rules! assert_hit {
    ($handled:expr) => {{
        if !$handled.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Endpoint has not been hit");
        }
    }};
}

#[macro_export]
macro_rules! assert_not_hit {
    ($handled:expr) => {{
        if $handled.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Endpoint has been hit");
        }
    }};
}
