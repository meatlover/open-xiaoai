mod server;
mod tls;

use neon::prelude::*;

mod runtime {
    use neon::prelude::*;
    use tokio::runtime::Runtime;

    pub fn runtime() -> Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio rt")
    }

    #[neon::main]
    pub fn main(mut cx: ModuleContext) -> NeonResult<()> {
        let _ = neon::set_global_executor(&mut cx, runtime());
        neon::registered().export(&mut cx)?;
        Ok(())
    }
}

#[neon::export]
pub async fn start() {
    server::AppServer::run().await;
}
