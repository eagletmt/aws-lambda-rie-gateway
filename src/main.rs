use futures::stream::TryStreamExt as _;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let make_service = hyper::service::make_service_fn(|_| async {
        Ok::<_, std::convert::Infallible>(hyper::service::service_fn(handle))
    });
    let server = if let Some(listener) = listenfd::ListenFd::from_env().take_tcp_listener(0)? {
        hyper::server::Server::from_tcp(listener)?
    } else {
        let addr = "127.0.0.1:8080".parse()?;
        hyper::server::Server::bind(&addr)
    }
    .serve(make_service);
    server.await?;
    Ok(())
}

// https://docs.aws.amazon.com/apigateway/latest/developerguide/http-api-develop-integrations-lambda.html
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayV2LambdaProxyIntegrationV2<'a> {
    version: &'a str,
    raw_path: &'a str,
    raw_query_string: Option<&'a str>,
    headers: std::collections::HashMap<String, String>,
    query_string_parameters: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
    is_base64_encoded: bool,
    request_context: ApiGatewayV2LambdaProxyIntegrationV2RequestContext<'a>,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayV2LambdaProxyIntegrationV2RequestContext<'a> {
    http: ApiGatewayV2LambdaProxyIntegrationV2RequestContextHttp<'a>,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayV2LambdaProxyIntegrationV2RequestContextHttp<'a> {
    method: String,
    path: &'a str,
}
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayV2LambdaResponseV1 {
    is_base64_encoded: bool,
    status_code: u16,
    #[serde(default)]
    headers: std::collections::HashMap<Vec<u8>, String>,
    body: String,
}

async fn handle(
    request: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, anyhow::Error> {
    let query_string_parameters = if request.uri().query().is_some() {
        let u = url::Url::parse(&format!("{}", request.uri()))?;
        let mut params = std::collections::HashMap::new();
        for (k, v) in u.query_pairs() {
            params.insert(k.into_owned(), v.into_owned());
        }
        Some(params)
    } else {
        None
    };
    let method = request.method().clone();
    let uri = request.uri().clone();
    let mut headers = std::collections::HashMap::new();
    for (k, v) in request.headers() {
        headers.insert(k.as_str().to_owned(), v.to_str()?.to_owned());
    }
    let body = request
        .into_body()
        .map_ok(|b| bytes::BytesMut::from(&b[..]))
        .try_concat()
        .await?;
    let payload = ApiGatewayV2LambdaProxyIntegrationV2 {
        version: "2.0",
        raw_path: uri.path(),
        raw_query_string: uri.query(),
        headers,
        query_string_parameters,
        body: if body.is_empty() {
            None
        } else {
            Some(base64::encode(&body))
        },
        is_base64_encoded: true,
        request_context: ApiGatewayV2LambdaProxyIntegrationV2RequestContext {
            http: ApiGatewayV2LambdaProxyIntegrationV2RequestContextHttp {
                method: format!("{}", method),
                path: uri.path(),
            },
        },
    };
    log::info!(
        "Send upstream request: {}",
        serde_json::to_string(&payload)?
    );
    let resp = reqwest::Client::new()
        .post("http://localhost:9000/2015-03-31/functions/function/invocations")
        .json(&payload)
        .send()
        .await?;
    let lambda_response: ApiGatewayV2LambdaResponseV1 = resp.json().await?;
    log::info!("Received upstream response: {:?}", lambda_response);

    let mut builder = hyper::Response::builder().status(lambda_response.status_code);
    for (k, v) in lambda_response.headers {
        builder = builder.header(k.as_slice(), v);
    }
    Ok(builder.body(hyper::Body::from(lambda_response.body))?)
}
