use crate::provider::aws::nested_references as nested;
use serde_json::Value;
const BRANCHES: &[&str] = &[
    "doc_db_settings",
    "ibm_db2_luw_settings",
    "ibm_db2_zos_settings",
    "maria_db_settings",
    "microsoft_sql_server_settings",
    "mongo_db_settings",
    "mysql_settings",
    "oracle_settings",
    "postgresql_settings",
    "redshift_settings",
    "sybase_ase_settings",
];
fn branch(engine: &str) -> Option<&'static str> {
    Some(match engine {
        "docdb" => "doc_db_settings",
        "db2" => "ibm_db2_luw_settings",
        "db2-zos" => "ibm_db2_zos_settings",
        "mariadb" => "maria_db_settings",
        "sqlserver" => "microsoft_sql_server_settings",
        "mongodb" => "mongo_db_settings",
        "mysql" | "aurora" => "mysql_settings",
        "oracle" => "oracle_settings",
        "postgres" | "aurora-postgresql" => "postgresql_settings",
        "redshift" => "redshift_settings",
        "sybase" => "sybase_ase_settings",
        _ => return None,
    })
}
pub(super) fn field(path: &str) -> Option<(&'static str, &'static [&'static str], &'static str)> {
    let suffix = path.strip_prefix("settings.")?;
    let (block, field) = suffix.split_once('.')?;
    if !BRANCHES.contains(&block) {
        return None;
    }
    match field {
        "certificate_arn" if block != "redshift_settings" => Some((
            "aws_dms_certificate",
            &["certificate_arn"],
            "DMS provider certificate",
        )),
        "s3_access_role_arn"
            if !matches!(
                block,
                "doc_db_settings" | "mongo_db_settings" | "sybase_ase_settings"
            ) =>
        {
            Some(("aws_iam_role", &["arn"], "DMS provider S3 access role"))
        }
        "secrets_manager_oracle_asm_access_role_arn"
        | "secrets_manager_security_db_encryption_access_role_arn"
            if block == "oracle_settings" =>
        {
            Some(("aws_iam_role", &["arn"], "Oracle provider secret role"))
        }
        "secrets_manager_oracle_asm_secret_id"
        | "secrets_manager_security_db_encryption_secret_id"
            if block == "oracle_settings" =>
        {
            Some((
                "aws_secretsmanager_secret",
                &["arn", "id", "name"],
                "Oracle provider secret",
            ))
        }
        _ => None,
    }
}
pub(super) fn qualify(e: &[(&Value, &Value)], t: usize, p: &str) -> bool {
    let (v, m) = e[t];
    let Some(engine) = nested::value(v, m, "engine")
        .and_then(Value::as_str)
        .and_then(branch)
    else {
        return false;
    };
    nested::singleton(v, m, "settings")
        && p.split('.').nth(2) == Some(engine)
        && nested::singleton(v, m, &format!("settings.0.{engine}"))
        && BRANCHES.iter().filter(|&&b| b != engine).all(|b| {
            nested::value(v, m, &format!("settings.0.{b}"))
                .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
        })
}
