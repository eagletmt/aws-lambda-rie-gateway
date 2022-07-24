# aws-lambda-rie-gateway
Convert HTTP request to API Gateway payload for [aws-lambda-rie](https://github.com/aws/aws-lambda-runtime-interface-emulator)

# Usage
1. Start Docker container for Lambda with aws-lambda-rie: `docker run -p 9000:8080 0123456789012dkr.ecr.ap-northeast-1.amazonaws.com/your-awesome-app:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef`
    - See documents and README of aws-lambda-rie for details
        - https://docs.aws.amazon.com/lambda/latest/dg/images-test.html
        - https://github.com/aws/aws-lambda-runtime-interface-emulator
2. Start aws-lambda-rie-gateway: `cargo run`
3. Then you can access Lambda for API Gateway with normal HTTP request: `curl http://localhost:8080/hello`

# Usage Docker Image
## From container registry
1. Run `docker run --rm --env TARGET_URL=http://rie_app:8080 --publish 8080:8080 public.ecr.aws/eagletmt/aws-lambda-rie-gateway`

## From source
1. Clone this repository
2. Then `docker build --tag aws-lambda-rie-gateway`
3. Execute with `docker run --rm --env TARGET_URL=http://rie_app:8080 --publish 8080:8080 aws-lambda-rie-gateway`
