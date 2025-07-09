use super::{
    Config,
    context::{SalarymanDContext, StdinBuffer},
};
use dropshot::{HttpError, HttpResponseOk, RequestContext, TypedBody, endpoint};
use std::sync::Arc;

#[endpoint {
    method = GET,
    path = "/config",
}]
pub async fn endpoint_get_config(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
) -> Result<HttpResponseOk<Config>, HttpError> {
    Ok(HttpResponseOk(rqctx.context().config.clone()))
}

#[endpoint {
    method = PUT,
    path = "/services/write"
}]
pub async fn endpoint_post_stdin(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
    update: TypedBody<StdinBuffer>,
) -> Result<HttpResponseOk<()>, HttpError> {
    let ctx = rqctx.context();
    let stdin_str = update.into_inner();
    for i in 0..ctx.service.len() {
        let mut lock = ctx.service[i].lock().await;
        if lock.started().await {
            lock.writeln_stdin(stdin_str.string.clone()).await.unwrap(); //TODO: PROPERLY HANDLE ERROR!
        }
        drop(lock);
    }

    Ok(HttpResponseOk(()))
}
