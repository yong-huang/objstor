pub const AWS_AUTH_HEADER: &str = "authorization";
pub const AWS_DATE_HEADER: &str = "x-amz-date";
pub const AWS_CONTENT_SHA256_HEADER: &str = "x-amz-content-sha256";
pub const AWS_HOST_HEADER: &str = "host";

pub fn is_aws_auth_request(headers: &axum::http::HeaderMap) -> bool {
    headers.get(AWS_AUTH_HEADER).is_some()
}
