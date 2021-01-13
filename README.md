# aws-lambda-rie-gateway
Convert HTTP request to API Gateway payload for [aws-lambda-rie](https://github.com/aws/aws-lambda-runtime-interface-emulator)

# Usage
1. Start Docker container for Lambda with aws-lambda-rie: `docker run -p 9000:8080 0123456789012dkr.ecr.ap-northeast-1.amazonaws.com/your-awesome-app:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef`
    - See documents and README of aws-lambda-rie for details
        - https://docs.aws.amazon.com/lambda/latest/dg/images-test.html
        - https://github.com/aws/aws-lambda-runtime-interface-emulator
2. Start aws-lambda-rie-gateway: `cargo run`
3. Then you can access Lambda for API Gateway with normal HTTP request: `curl http://localhost:8080/hello`
