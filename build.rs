#[path = "build/icon.rs"]
mod icon;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=assets/icons/aws");
    println!("cargo:rerun-if-changed=assets/icons/terraform");
    println!("cargo:rerun-if-changed=build/icon.rs");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    for entry in std::fs::read_dir("assets/icons/aws")? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "svg") {
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or("invalid artwork filename")?;
            let source = std::fs::read_to_string(&path)?;
            let symbol = icon::symbol(
                &source,
                &format!("planorama-icon-aws-service-{}", name.to_ascii_lowercase()),
            )
            .map_err(|error| format!("invalid bundled icon {name}: {error}"))?;
            std::fs::write(out.join(format!("service-{name}.svg")), symbol)?;
        }
    }
    for (file, id) in [
        ("Amazon-Cognito", "cognito"),
        ("AWS-Step-Functions", "step-functions"),
        ("Amazon-Redshift", "redshift"),
        ("AWS-CloudTrail", "cloudtrail"),
        ("AWS-Backup", "backup"),
        ("AWS-Organizations", "organizations"),
        ("Amazon-Virtual-Private-Cloud", "vpc"),
        ("Amazon-EC2", "ec2"),
        ("AWS-PrivateLink", "endpoint"),
        ("Elastic-Load-Balancing", "elb"),
        ("AWS-Lambda", "lambda"),
        ("Amazon-Simple-Queue-Service", "sqs"),
        ("Amazon-Kinesis-Data-Streams", "kinesis"),
        ("Amazon-Data-Firehose", "firehose"),
        ("AWS-Glue", "glue"),
        ("Amazon-Athena", "athena"),
        ("Amazon-Simple-Notification-Service", "sns"),
        ("Amazon-EventBridge", "eventbridge"),
        ("Amazon-Elastic-Container-Service", "ecs"),
        ("Amazon-Elastic-Container-Registry", "ecr"),
        ("Amazon-Elastic-Kubernetes-Service", "eks"),
        ("Amazon-RDS", "rds"),
        ("Amazon-DynamoDB", "dynamodb"),
        ("Amazon-ElastiCache", "cache"),
        ("Amazon-OpenSearch-Service", "search"),
        ("Amazon-Simple-Storage-Service", "s3"),
        ("Amazon-CloudFront", "cloudfront"),
        ("AWS-WAF", "waf"),
        ("Amazon-Route-53", "route53"),
        ("AWS-Certificate-Manager", "acm"),
        ("AWS-Private-Certificate-Authority", "private-ca"),
        ("AWS-Key-Management-Service", "kms"),
        ("AWS-Secrets-Manager", "secretsmanager"),
        ("AWS-Systems-Manager", "ssm"),
        ("Amazon-EFS", "efs"),
        ("Amazon-Elastic-Block-Store", "ebs"),
        ("Amazon-API-Gateway", "api"),
        ("AWS-Identity-and-Access-Management", "iam"),
        ("Amazon-CloudWatch", "monitoring"),
        ("Amazon-EC2-Auto-Scaling", "scaling"),
    ] {
        let source = std::fs::read_to_string(format!("assets/icons/aws/{file}.svg"))?;
        let symbol = icon::symbol(&source, &format!("planorama-icon-aws-{id}"))
            .map_err(|e| format!("invalid bundled icon {file}: {e}"))?;
        std::fs::write(out.join(format!("{file}.svg")), symbol)?;
    }
    let source = std::fs::read_to_string("assets/icons/terraform/terraform-builtin-24.svg")?;
    let symbol = icon::symbol(&source, "planorama-icon-terraform")
        .map_err(|e| format!("invalid bundled Terraform icon: {e}"))?;
    std::fs::write(out.join("terraform-builtin-24.svg"), symbol)?;
    Ok(())
}
