use super::Config;
use super::context::{SalarymanDContext, SalarymanService};
use dropshot::{HttpError, HttpResponseOk, Path, RequestContext, TypedBody, endpoint};
use salaryman::model::{NewService, ServicePath, StdinBuffer, UpdateConf};
use salaryman::service::{Service, ServiceConf};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

#[endpoint {
    method = GET,
    path = "/config",
}]
pub async fn endpoint_get_config(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
) -> Result<HttpResponseOk<Config>, HttpError> {
    let ctx = rqctx.context();
    let lock = ctx.config.read().await;
    let conf = lock.clone();
    drop(lock);
    Ok(HttpResponseOk(conf))
}
#[endpoint {
    method = PUT,
    path = "/config",
}]
pub async fn endpoint_put_config(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
    body: TypedBody<UpdateConf>,
) -> Result<HttpResponseOk<()>, HttpError> {
    let ctx = rqctx.context();
    let update = body.into_inner();
    if let Some(addr) = update.address {
        let mut lock = ctx.config.write().await;
        lock.address = Some(addr);
        drop(lock);
    }
    if let Some(port) = update.port {
        let mut lock = ctx.config.write().await;
        lock.port = Some(port);
        drop(lock);
    }
    Ok(HttpResponseOk(()))
}
#[endpoint {
    method = GET,
    path = "/config/save"
}]
pub async fn endpoint_get_config_save(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
) -> Result<HttpResponseOk<()>, HttpError> {
    let ctx = rqctx.context();
    let mut v: Vec<ServiceConf> = Vec::new();
    let rlock = ctx.services.read().await;
    for i in 0..rlock.len() {
        v.push(rlock[i].config.clone());
    }
    drop(rlock);
    let rlock = ctx.config.read().await;
    let save = Config {
        address: rlock.address,
        port: rlock.port,
        user: rlock.user.clone(),
        service: v,
    };
    drop(rlock);
    let mut f = match File::open(&ctx.save_file).await {
        Ok(f) => f,
        Err(_) => {
            return Err(HttpError::for_internal_error(String::from(
                "cannot open desired file",
            )));
        }
    };
    let save = match toml::to_string(&save) {
        Ok(s) => s,
        Err(_) => {
            return Err(HttpError::for_internal_error(String::from(
                "cannot serialize config!",
            )));
        }
    };
    match f.write_all(save.as_str().as_bytes()).await {
        Ok(_) => (),
        Err(_) => {
            return Err(HttpError::for_internal_error(String::from(
                "could not write to file!",
            )));
        }
    }
    Ok(HttpResponseOk(()))
}
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
        let lock = ctx.services.read().await;
        for i in 0..lock.len() {
            v.push(lock[i].config.clone());
        }
        v
    };
    Ok(HttpResponseOk(ret))
}
#[endpoint {
    method = POST,
    path = "/service",
}]
pub async fn endpoint_post_service(
    rqctx: RequestContext<Arc<SalarymanDContext>>,
    body: TypedBody<NewService>,
) -> Result<HttpResponseOk<ServiceConf>, HttpError> {
    let ctx = rqctx.context();
    let body = body.into_inner();
    let mut s: ServiceConf = ServiceConf::new();
    if let Some(name) = &body.name {
        s.name = name.clone().to_owned();
    } else {
        return Err(HttpError::for_bad_request(
            None,
            String::from("name field is required!"),
        ));
    }
    if let Some(command) = &body.command {
        s.command = command.clone().to_owned();
    } else {
        return Err(HttpError::for_bad_request(
            None,
            String::from("command field is required!"),
        ));
    }
    if let Some(args) = &body.args {
        if let Some(args) = args {
            s.args = Some(args.clone().to_owned());
        }
    }
    if let Some(dir) = &body.directory {
        if let Some(dir) = dir {
            s.directory = Some(dir.clone().to_owned());
        }
    }
    if let Some(auto) = &body.autostart {
        s.autostart = auto.clone().to_owned();
    } else {
        s.autostart = false;
    }
    let service: SalarymanService =
        SalarymanService::from_parts(s.clone(), Arc::new(RwLock::new(Service::from_conf(&s))));
    if service.config.autostart {
        let mut lock = service.service.write().await;
        match lock.start_with_output().await {
            Ok(_) => (),
            Err(_) => (),
        }
        drop(lock);
    }
    let mut lock = ctx.config.write().await;
    lock.service.push(s.clone());
    drop(lock);
    let mut lock = ctx.services.write().await;
    lock.push(Arc::new(service));
    drop(lock);
    Ok(HttpResponseOk(ServiceConf::new()))
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
    let lock = ctx.services.read().await;
    for i in 0..lock.len() {
        if lock[i].config.uuid == u {
            service = Some(lock[i].clone());
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
    let lock = ctx.services.read().await;
    for i in 0..lock.len() {
        if lock[i].config.uuid == u {
            service = Some(lock[i].clone());
        } else {
            continue;
        }
    }
    match service {
        Some(s) => {
            let mut lock = s.service.write().await;
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
    let lock = ctx.services.read().await;
    for i in 0..lock.len() {
        if lock[i].config.uuid == u {
            service = Some(lock[i].clone());
        } else {
            continue;
        }
    }
    match service {
        Some(s) => {
            let mut lock = s.service.write().await;
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
    let lock = ctx.services.read().await;
    for i in 0..lock.len() {
        if lock[i].config.uuid == u {
            service = Some(lock[i].clone());
        } else {
            continue;
        }
    }
    match service {
        Some(s) => {
            let mut lock = s.service.write().await;
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
    let lock = ctx.services.read().await;
    for i in 0..lock.len() {
        if lock[i].config.uuid == u {
            service = Some(lock[i].clone());
        } else {
            continue;
        }
    }
    match service {
        Some(s) => {
            let mut lock = s.service.write().await;
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
