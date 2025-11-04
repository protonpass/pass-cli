use crate::PassClient;
pub use crate::PlanType;
use crate::common::{CodeResponse, SUCCESS_CODE};
use crate::test_tools::client_features::TestClientFeatures;
use crate::test_tools::{TEST_PASSPHRASE, init_session, setup_user_access};
pub use muon::Method;
use muon::test::server::{Request, Response, Server};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub trait MuonServerExt {
    fn handler<P, F>(&self, path: P, handler: F) -> Arc<AtomicBool>
    where
        P: AsRef<str> + Send + Sync + 'static,
        F: Fn(&Request) -> Option<Response> + Send + Sync + 'static;
    fn handler_with_method<P, F>(&self, method: Method, path: P, handler: F) -> Arc<AtomicBool>
    where
        P: AsRef<str> + Send + Sync + 'static,
        F: Fn(&Request) -> Option<Response> + Send + Sync + 'static;

    async fn pass_client(&self) -> PassClient;
    async fn pass_client_with_plan(&self, plan: PlanType) -> PassClient;
    async fn pass_client_no_setup(&self) -> PassClient;
}

impl MuonServerExt for Arc<Server> {
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

    async fn pass_client(&self) -> PassClient {
        self.pass_client_with_plan(PlanType::Free).await
    }

    async fn pass_client_with_plan(&self, plan: PlanType) -> PassClient {
        super::setup_user_data::setup(self);
        let client = self.pass_client_no_setup().await;

        client
            .setup_key_passphrases(TEST_PASSPHRASE)
            .await
            .expect("Error setting up passphrases");
        setup_user_access(self, plan);
        client
    }

    async fn pass_client_no_setup(&self) -> PassClient {
        let key = pass_domain::crypto::generate_encryption_key();
        let client = self.client().await;
        let session = client
            .new_session_without_credentials(())
            .await
            .expect("Error creating session");
        init_session(self, session).await;
        PassClient::new(client, Arc::new(TestClientFeatures::new(key)))
    }
}

#[derive(serde::Serialize)]
pub struct Empty;

pub fn success<R: serde::Serialize>(res: R) -> Option<Response> {
    let body = serde_json::to_vec(&res).unwrap();
    Some(
        Response::builder()
            .status(200)
            .body(axum_core::body::Body::from(body))
            .unwrap(),
    )
}

pub fn success_code() -> Option<Response> {
    let body = serde_json::to_vec(&CodeResponse { code: SUCCESS_CODE }).unwrap();
    Some(
        Response::builder()
            .status(200)
            .body(axum_core::body::Body::from(body))
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
