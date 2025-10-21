use crate::PassClient;
use muon::GET;

impl PassClient {
    pub async fn ping(&self) -> anyhow::Result<()> {
        info!(">>> Sending ping");
        let res = self.send(GET!("/tests/ping")).await?.ok()?;
        info!("<<< Ping {}", res.status());

        Ok(())
    }
}
