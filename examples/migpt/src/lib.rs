use neon::prelude::*;
use node::NodeManager;

use runtime::runtime;

mod node;
mod runtime;

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    let _ = neon::set_global_executor(&mut cx, runtime());
    neon::registered().export(&mut cx)?;
    NodeManager::instance().init(cx);
    Ok(())
}
