use aws_lambda_events::apigw::{
    ApiGatewayV2httpRequest, ApiGatewayV2httpRequestContext,
    ApiGatewayV2httpRequestContextHttpDescription, ApiGatewayV2httpResponse,
};
use base64::decode;
use chrono::Utc;
use clap::Parser as _;
use futures::stream::TryStreamExt as _;

#[derive(Debug, clap::Parser)]
struct Opt {
    /// Bind address
    #[clap(short, long, env, default_value = "127.0.0.1:8080")]
    bind: String,
    /// Target root URL of RIE
    #[clap(short, long, env, default_value = "http://localhost:9000")]
    target_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    let Opt { bind, target_url } = Opt::parse();

    let make_service = hyper::service::make_service_fn(move |_| {
        let target_url = target_url.clone();
        async {
            Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |r| {
                handle(target_url.clone(), r)
            }))
        }
    });
    let server = if let Some(listener) = listenfd::ListenFd::from_env().take_tcp_listener(0)? {
        tracing::info!("Listen {}", listener.local_addr()?);
        hyper::server::Server::from_tcp(listener)?
    } else {
        let addr = bind.parse()?;
        tracing::info!("Listen {}", addr);
        hyper::server::Server::bind(&addr)
    }
    .serve(make_service)
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("Shutting down...");
    });
    server.await?;
    Ok(())
}

async fn handle(
    target_url: String,
    request: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, anyhow::Error> {
    let query_string_parameters = if request.uri().query().is_some() {
        let u = url::Url::parse(&format!("{}", request.uri()))?;
        let mut params = std::collections::HashMap::new();
        for (k, v) in u.query_pairs() {
            params.insert(k.into_owned(), v.into_owned());
        }
        params
    } else {
        std::collections::HashMap::new()
    };
    let method = request.method().clone();
    let uri = request.uri().clone();
    let protocol = request.version();
    let headers = request.headers().clone();

    let body = request
        .into_body()
        .map_ok(|b| bytes::BytesMut::from(&b[..]))
        .try_concat()
        .await?;

    let datetime = Utc::now();

    let payload = ApiGatewayV2httpRequest {
        version: Some("2.0".to_owned()),
        route_key: None,
        raw_path: Some(uri.path().to_owned()),
        raw_query_string: uri.query().map(|s| s.to_owned()),
        cookies: None,
        headers: headers.clone(),
        query_string_parameters: query_string_parameters.into(),
        path_parameters: std::collections::HashMap::new(),
        request_context: ApiGatewayV2httpRequestContext {
            route_key: Some("$default".to_owned()),
            account_id: Some(String::new()),
            stage: Some("$default".to_owned()),
            request_id: Some(String::new()),
            authorizer: None,
            apiid: Some(String::new()),
            domain_name: Some(String::new()),
            domain_prefix: Some(String::new()),
            http: ApiGatewayV2httpRequestContextHttpDescription {
                method,
                path: Some(uri.path().to_owned()),
                protocol: Some(format!("{:?}", protocol)),
                source_ip: None,
                user_agent: None,
            },
            authentication: None,
            time: Some(datetime.format("%d/%b/%Y:%T %z").to_string()),
            time_epoch: datetime.timestamp_millis(),
        },
        stage_variables: std::collections::HashMap::new(),
        body: if body.is_empty() {
            None
        } else {
            Some(base64::encode(&body))
        },
        is_base64_encoded: true,
    };

    tracing::info!(
        "Send upstream request: {}",
        serde_json::to_string(&payload)?
    );

    let resp = reqwest::Client::new()
        .post(&format!(
            "{}/2015-03-31/functions/function/invocations",
            target_url
        ))
        .json(&payload)
        .send()
        .await?;

    let lambda_response: ApiGatewayV2httpResponse = resp.json().await.map_err(|e| {
        tracing::error!("{e}");
        e
    })?;
    tracing::info!("Received upstream response: {:?}", lambda_response);

    let status = hyper::StatusCode::from_u16(lambda_response.status_code as u16)?;
    let mut builder = hyper::Response::builder().status(status);
    let headers = builder.headers_mut().unwrap();
    *headers = lambda_response.headers;

    let body: Vec<u8> = if let Some(body) = lambda_response.body {
        body.as_ref().into()
    } else {
        Vec::new()
    };

    match lambda_response.is_base64_encoded {
        Some(value) => {
            if value {
                match decode(&body) {
                    Ok(decoded_bytes) => {
                        // Use the decoded bytes as needed
                        Ok(builder.body(hyper::Body::from(decoded_bytes))?)
                    }
                    Err(e) => {
                        tracing::warn!("Lambda response signaled it was base64, but could not decode it: {}", e);
                        Ok(builder.body(hyper::Body::from(body))?)
                    }
                }
            } else {
                Ok(builder.body(hyper::Body::from(body))?)
            }
        }
        None => {
            Ok(builder.body(hyper::Body::from(body))?)
        }
    }
}
