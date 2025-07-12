use super::context::{SalarymanDContext, SalarymanService, ServicePath, StdinBuffer};
use dropshot::{HttpError, HttpResponseOk, Path, RequestContext, TypedBody, endpoint};
use salaryman::service::ServiceConf;
use std::sync::Arc;

#[endpoint {
    method = GET,
    path = "/service",
}]
pub async fn endpoint_get_services(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
) -> Result<HttpResponseOk<Vec<ServiceConf>>, HttpError> {
    let ret = {
        let ctx = rqctx.context();
        let mut v: Vec<ServiceConf> = Vec::new();
        for i in 0..ctx.services.len() {
            v.push(ctx.services[i].config.clone());
        }
        v
    };
    Ok(HttpResponseOk(ret))
}
#[endpoint {
    method = GET,
    path = "/service/{service_uuid}",
}]
pub async fn endpoint_get_service(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
    path_params: Path<ServicePath>,
) -> Result<HttpResponseOk<ServiceConf>, HttpError> {
    let u = path_params.into_inner().service_uuid;
    let ctx = rqctx.context();
    let mut service: Option<Arc<SalarymanService>> = None;
    for i in 0..ctx.services.len() {
        if ctx.services[i].config.uuid == u {
            service = Some(ctx.services[i].clone());
        } else {
            continue;
        }
    }
    let s = match service {
        Some(s) => s.config.clone(),
        None => {
            return Err(HttpError::for_unavail(
                None,
                String::from("Service Not Found"),
            ));
        }
    };
    Ok(HttpResponseOk(s))
}
#[endpoint {
    method = GET,
    path = "/service/{service_uuid}/start",
}]
pub async fn endpoint_start_service(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
    path_params: Path<ServicePath>,
) -> Result<HttpResponseOk<()>, HttpError> {
    let u = path_params.into_inner().service_uuid;
    let ctx = rqctx.context();
    let mut service: Option<Arc<SalarymanService>> = None;
    for i in 0..ctx.services.len() {
        if ctx.services[i].config.uuid == u {
            service = Some(ctx.services[i].clone());
        } else {
            continue;
        }
    }
    match service {
        Some(s) => {
            let mut lock = s.service.lock().await;
            match lock.start_with_output().await {
                Ok(_) => (),
                Err(e) => return Err(HttpError::for_internal_error(e.to_string())),
            }
        }
        None => {
            return Err(HttpError::for_unavail(
                None,
                String::from("Service Not Found"),
            ));
        }
    };
    Ok(HttpResponseOk(()))
}
#[endpoint {
    method = GET,
    path = "/service/{service_uuid}/stop",
}]
pub async fn endpoint_stop_service(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
    path_params: Path<ServicePath>,
) -> Result<HttpResponseOk<()>, HttpError> {
    let u = path_params.into_inner().service_uuid;
    let ctx = rqctx.context();
    let mut service: Option<Arc<SalarymanService>> = None;
    for i in 0..ctx.services.len() {
        if ctx.services[i].config.uuid == u {
            service = Some(ctx.services[i].clone());
        } else {
            continue;
        }
    }
    match service {
        Some(s) => {
            let mut lock = s.service.lock().await;
            match lock.stop().await {
                Ok(_) => (),
                Err(e) => return Err(HttpError::for_internal_error(e.to_string())),
            }
        }
        None => {
            return Err(HttpError::for_unavail(
                None,
                String::from("Service Not Found"),
            ));
        }
    };
    Ok(HttpResponseOk(()))
}
#[endpoint {
    method = GET,
    path = "/service/{service_uuid}/restart",
}]
pub async fn endpoint_restart_service(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
    path_params: Path<ServicePath>,
) -> Result<HttpResponseOk<()>, HttpError> {
    let u = path_params.into_inner().service_uuid;
    let ctx = rqctx.context();
    let mut service: Option<Arc<SalarymanService>> = None;
    for i in 0..ctx.services.len() {
        if ctx.services[i].config.uuid == u {
            service = Some(ctx.services[i].clone());
        } else {
            continue;
        }
    }
    match service {
        Some(s) => {
            let mut lock = s.service.lock().await;
            match lock.restart_with_output().await {
                Ok(_) => (),
                Err(e) => return Err(HttpError::for_internal_error(e.to_string())),
            }
        }
        None => {
            return Err(HttpError::for_unavail(
                None,
                String::from("Service Not Found"),
            ));
        }
    };
    Ok(HttpResponseOk(()))
}
#[endpoint {
    method = PUT,
    path = "/service/{service_uuid}/write"
}]
pub async fn endpoint_post_stdin(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
    path_params: Path<ServicePath>,
    update: TypedBody<StdinBuffer>,
) -> Result<HttpResponseOk<()>, HttpError> {
    let ctx = rqctx.context();
    let stdin_str = update.into_inner();
    let u = path_params.into_inner().service_uuid;
    let mut service: Option<Arc<SalarymanService>> = None;
    for i in 0..ctx.services.len() {
        if ctx.services[i].config.uuid == u {
            service = Some(ctx.services[i].clone());
        } else {
            continue;
        }
    }
    match service {
        Some(s) => {
            let mut lock = s.service.lock().await;
            if lock.started().await {
                let b = if let Some(endl) = stdin_str.endl {
                    if endl {
                        lock.writeln_stdin(stdin_str.stdin.clone()).await //TODO: PROPERLY HANDLE ERROR!
                    } else {
                        lock.write_stdin(stdin_str.stdin.clone()).await //TODO: PROPERLY HANDLE ERROR!
                    }
                } else {
                    lock.writeln_stdin(stdin_str.stdin.clone()).await //TODO: PROPERLY HANDLE ERROR!
                };
                match b {
                    Ok(_) => (),
                    Err(e) => return Err(HttpError::for_internal_error(e.to_string())),
                }
            }
            drop(lock);
        }
        None => {
            return Err(HttpError::for_unavail(
                None,
                String::from("Service Not Found"),
            ));
        }
    }
    Ok(HttpResponseOk(()))
}
