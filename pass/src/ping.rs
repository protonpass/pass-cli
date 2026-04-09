use crate::{PassClient, PassClientContext};
use muon::GET;

impl<C: PassClientContext> PassClient<C> {
    pub async fn ping(&self) -> anyhow::Result<()> {
        info!(">>> Sending ping");
        let res = self.send(GET!("/tests/ping")).await?.ok()?;
        info!("<<< Ping {}", res.status());

        Ok(())
    }
}
