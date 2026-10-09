#[path = "build/icon.rs"]
mod icon;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=assets/icons/aws");
    println!("cargo:rerun-if-changed=build/icon.rs");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    for (file, id) in [
        ("Amazon-Virtual-Private-Cloud", "vpc"),
        ("Amazon-EC2", "ec2"),
        ("AWS-PrivateLink", "endpoint"),
        ("Elastic-Load-Balancing", "elb"),
        ("AWS-Lambda", "lambda"),
        ("Amazon-Elastic-Container-Service", "ecs"),
        ("Amazon-Elastic-Kubernetes-Service", "eks"),
        ("Amazon-RDS", "rds"),
        ("Amazon-DynamoDB", "dynamodb"),
        ("Amazon-ElastiCache", "cache"),
        ("Amazon-OpenSearch-Service", "search"),
        ("Amazon-Simple-Storage-Service", "s3"),
        ("Amazon-CloudFront", "cloudfront"),
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
    Ok(())
}
