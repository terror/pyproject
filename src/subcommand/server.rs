use super::*;

pub(crate) async fn run() -> Result {
  Server::run().await;
  Ok(())
}
