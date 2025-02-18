// This code was derived from the hudsucker repository:
// https://github.com/omjadas/hudsucker

use async_trait::async_trait;
use http::{Request, Response};
use hyper::{body::to_bytes, Body};
pub use proxyapi_models::{ProxiedRequest, ProxiedResponse};
use std::sync::mpsc::SyncSender;

use crate::{HttpContext, HttpHandler, RequestResponse};

#[derive(Clone, Debug)]
pub struct ProxyHandler {
    tx: SyncSender<ProxyHandler>,
    req: Option<ProxiedRequest>,
    res: Option<ProxiedResponse>,
}

impl ProxyHandler {
    pub fn new(tx: SyncSender<ProxyHandler>) -> Self {
        Self {
            tx,
            req: None,
            res: None,
        }
    }

    pub fn to_parts(self) -> (Option<ProxiedRequest>, Option<ProxiedResponse>) {
        (self.req, self.res)
    }

    pub fn set_req(&mut self, req: ProxiedRequest) -> Self {
        Self {
            tx: self.clone().tx,
            req: Some(req),
            res: None,
        }
    }

    pub fn set_res(&mut self, res: ProxiedResponse) -> Self {
        Self {
            tx: self.clone().tx,
            req: self.clone().req,
            res: Some(res),
        }
    }

    pub fn send_output(self) {
        if let Err(e) = self.tx.send(self.clone()) {
            eprintln!("Error on sending Response to main thread: {}", e);
        }
    }

    pub fn req(&self) -> &Option<ProxiedRequest> {
        &self.req
    }

    pub fn res(&self) -> &Option<ProxiedResponse> {
        &self.res
    }

    pub fn handle_request(&self, _ctx: &HttpContext, req: Request<Body>) {
        println!("\n=== 新请求 ===");
        println!(">>> 方法: {}", req.method());
        println!(">>> 路径: {}", req.uri());
        println!(">>> 请求头:");
        for (name, value) in req.headers() {
            println!("    {}: {}", name, value.to_str().unwrap_or("无法解析的值"));
        }
        println!("===============");
    }

    pub fn handle_response(&self, _ctx: &HttpContext, res: Response<Body>) {
        println!("\n=== 响应详情 ===");
        println!("<<< 状态: {}", res.status());
        println!("<<< 响应头:");
        for (name, value) in res.headers() {
            println!("    {}: {}", name, value.to_str().unwrap_or("无法解析的值"));
        }
        println!("===============");
    }
}

#[async_trait]
impl HttpHandler for ProxyHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        mut req: Request<Body>,
    ) -> RequestResponse {
        println!("\n=== 新请求 ===");
        println!(">>> 方法: {}", req.method());
        println!(">>> 完整 URL: {}", req.uri());
        println!(">>> 协议版本: {:?}", req.version());
        println!(">>> 请求头:");
        for (name, value) in req.headers() {
            println!("    {}: {}", name, value.to_str().unwrap_or("无法解析的值"));
        }

        // 尝试读取和打印请求体
        let mut body_mut = req.body_mut();
        let body_bytes = to_bytes(&mut body_mut).await.unwrap_or_default();
        if !body_bytes.is_empty() {
            if let Ok(body_str) = String::from_utf8(body_bytes.to_vec()) {
                println!(">>> 请求体:\n{}", body_str);
            }
        }
        println!("===============");

        *body_mut = Body::from(body_bytes.clone());

        let output_request = ProxiedRequest::new(
            req.method().clone(),
            req.uri().clone(),
            req.version(),
            req.headers().clone(),
            body_bytes,
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );
        *self = self.set_req(output_request);

        req.into()
    }

    async fn handle_response(
        &mut self,
        _ctx: &HttpContext,
        mut res: Response<Body>,
    ) -> Response<Body> {
        println!("\n=== 响应详情 ===");
        println!("<<< 状态: {}", res.status());
        println!("<<< 响应头:");
        for (name, value) in res.headers() {
            println!("    {}: {}", name, value.to_str().unwrap_or("无法解析的值"));
        }
        println!("===============");

        let mut body_mut = res.body_mut();
        let body_bytes = to_bytes(&mut body_mut).await.unwrap_or_default();
        *body_mut = Body::from(body_bytes.clone());

        let output_response = ProxiedResponse::new(
            res.status(),
            res.version(),
            res.headers().clone(),
            body_bytes,
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );

        self.set_res(output_response).send_output();

        res
    }
}
