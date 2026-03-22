use tracing::info;

pub fn log_audit(operation: &str, resource: &str, user: &str, result: bool) {
    info!(
        timestamp = %chrono::Utc::now(),
        operation = %operation,
        resource = %resource,
        user = %user,
        result = %result,
        target = "audit_log",
    );
}
