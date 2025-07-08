use muon::Client;

pub async fn run(client: Client) {
    client.logout().await;
    info!("Logged out client");
}
