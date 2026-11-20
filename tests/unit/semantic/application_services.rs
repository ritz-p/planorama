use super::label;
use crate::configured_service_links::{check, check_provider_alias, verified};
use serde_json::{Value, json};

const PROVIDER: &str = "registry.terraform.io/hashicorp/aws";

fn plan(resources: &[(&str, Value, Value)]) -> Value {
    let changes: Vec<_> = resources
        .iter()
        .map(|(address, values, _)| {
            let (kind, name) = address.split_once('.').unwrap();
            json!({
                "address": address, "mode": "managed", "type": kind, "name": name,
                "provider_name": PROVIDER, "change": {"actions": ["create"], "after": values}
            })
        })
        .collect();
    let config: Vec<_> = resources
        .iter()
        .map(|(address, _, expressions)| {
            let (kind, name) = address.split_once('.').unwrap();
            json!({
                "address": address, "mode": "managed", "type": kind, "name": name,
                "provider_config_key": "aws", "expressions": expressions
            })
        })
        .collect();
    json!({
        "format_version": "1.2", "terraform_version": "1.9.8", "resource_changes": changes,
        "configuration": {
            "provider_config": {"aws": {"name": "aws", "full_name": PROVIDER, "expressions": {
                "region": {"constant_value": "us-east-1"},
                "assume_role": [{"role_arn": {"constant_value": "arn:aws:iam::123456789012:role/Provider"}}]
            }}},
            "root_module": {"resources": config}
        }
    })
}

fn graph(plan: &Value) -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(&plan.to_string()).unwrap())
}

fn amplify() -> Value {
    let service = "arn:aws:iam::123456789012:role/Build";
    let compute = "arn:aws:iam::123456789012:role/Compute";
    let backend = "arn:aws:amplify:us-east-1:123456789012:apps/d123/backendenvironments/dev";
    plan(&[
        ("aws_iam_role.build", json!({"arn": service}), json!({})),
        ("aws_iam_role.compute", json!({"arn": compute}), json!({})),
        (
            "aws_amplify_app.site",
            json!({"id":"d123", "iam_service_role_arn":service, "compute_role_arn":compute}),
            json!({
                "iam_service_role_arn":{"references":["aws_iam_role.build.arn"]},
                "compute_role_arn":{"references":["aws_iam_role.compute.arn"]}
            }),
        ),
        (
            "aws_amplify_backend_environment.dev",
            json!({"app_id":"d123", "environment_name":"dev", "arn":backend}),
            json!({"app_id":{"references":["aws_amplify_app.site.id"]}}),
        ),
        (
            "aws_amplify_branch.trunk",
            json!({"app_id":"d123", "branch_name":"trunk", "backend_environment_arn":backend}),
            json!({
                "app_id":{"references":["aws_amplify_app.site.id"]},
                "backend_environment_arn":{"references":["aws_amplify_backend_environment.dev.arn"]}
            }),
        ),
    ])
}

#[test]
fn amplify_uses_separate_roles_and_same_app_backend() {
    let fixture = amplify();
    let input = fixture.to_string();
    for (address, path, source, field) in [
        (
            "aws_amplify_app.site",
            "iam_service_role_arn",
            "aws_iam_role.build",
            "arn",
        ),
        (
            "aws_amplify_app.site",
            "compute_role_arn",
            "aws_iam_role.compute",
            "arn",
        ),
        (
            "aws_amplify_backend_environment.dev",
            "app_id",
            "aws_amplify_app.site",
            "id",
        ),
        (
            "aws_amplify_branch.trunk",
            "backend_environment_arn",
            "aws_amplify_backend_environment.dev",
            "arn",
        ),
    ] {
        check(
            &input,
            crate::plan::parse,
            label,
            &[(address, path)],
            (source, field),
        );
        check_provider_alias(
            &input,
            address,
            path,
            source,
            label(address.split('.').next().unwrap(), path).unwrap(),
            !source.starts_with("aws_iam_role."),
        );
    }
    let graph = graph(&fixture);
    for node in graph.nodes.iter().filter(|n| {
        matches!(
            n.resource_type.as_str(),
            super::AMPLIFY_BRANCH | super::AMPLIFY_BACKEND
        )
    }) {
        assert_eq!(node.member_owner.as_ref().unwrap().attribute, "app_id");
    }
}

#[test]
fn amplify_rejects_cross_app_and_computed_endpoints() {
    let mut fixture = amplify();
    fixture["resource_changes"][3]["change"]["after"]["app_id"] = json!("different");
    let graph = graph(&fixture);
    assert!(!verified(
        &graph,
        "aws_amplify_branch.trunk",
        "backend_environment_arn",
        Some("Amplify backend environment")
    ));
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_amplify_backend_environment.dev")
            .unwrap()
            .member_owner
            .is_none()
    );
    for (kind, path) in [
        (super::AMPLIFY_BRANCH, "associated_resources"),
        (super::AMPLIFY_BRANCH, "source_branch"),
        (super::AMPLIFY_BRANCH, "destination_branch"),
        (super::AMPLIFY_BACKEND, "stack_name"),
        (super::AMPLIFY_BACKEND, "deployment_artifacts"),
    ] {
        assert!(label(kind, path).is_none());
    }
}

#[test]
fn amplify_domain_reconciles_unordered_branches_and_custom_certificate() {
    let certificate = "arn:aws:acm:us-east-1:123456789012:certificate/demo";
    let mut fixture = plan(&[
        ("aws_amplify_app.site", json!({"id":"d123"}), json!({})),
        (
            "aws_amplify_branch.one",
            json!({"app_id":"d123","branch_name":"one"}),
            json!({"app_id":{"references":["aws_amplify_app.site.id"]}}),
        ),
        (
            "aws_amplify_branch.two",
            json!({"app_id":"d123","branch_name":"two"}),
            json!({"app_id":{"references":["aws_amplify_app.site.id"]}}),
        ),
        (
            "aws_acm_certificate.custom",
            json!({"arn":certificate}),
            json!({}),
        ),
        (
            "aws_amplify_domain_association.site",
            json!({
                "app_id":"d123", "sub_domain":[{"branch_name":"two","prefix":"docs"},{"branch_name":"one","prefix":""}],
                "certificate_settings":[{"type":"CUSTOM","custom_certificate_arn":certificate}]
            }),
            json!({
                "app_id":{"references":["aws_amplify_app.site.id"]},
                "sub_domain":[{"branch_name":{"references":["aws_amplify_branch.one.branch_name"]}},{"branch_name":{"references":["aws_amplify_branch.two.branch_name"]}}],
                "certificate_settings":[{"type":{"constant_value":"CUSTOM"},"custom_certificate_arn":{"references":["aws_acm_certificate.custom.arn"]}}]
            }),
        ),
        (
            "aws_amplify_webhook.trunk",
            json!({"app_id":"d123","branch_name":"one","url":"https://unused.invalid"}),
            json!({
                "app_id":{"references":["aws_amplify_app.site.id"]},
                "branch_name":{"references":["aws_amplify_branch.one.branch_name"]}
            }),
        ),
    ]);
    let domain = "aws_amplify_domain_association.site";
    let webhook = "aws_amplify_webhook.trunk";
    let cert_path = "certificate_settings.custom_certificate_arn";
    let branches = "sub_domain.branch_name";
    let proven = |plan: &Value, address: &str, path: &str| {
        verified(
            &graph(plan),
            address,
            path,
            label(address.split('.').next().unwrap(), path),
        )
    };
    for (address, path) in [
        (domain, branches),
        (domain, cert_path),
        (webhook, "branch_name"),
    ] {
        assert!(proven(&fixture, address, path), "{address}/{path}");
    }
    fixture["resource_changes"][4]["change"]["after"]["certificate_settings"][0]["type"] =
        json!("AMPLIFY_MANAGED");
    assert!(!proven(&fixture, domain, cert_path));
    fixture["resource_changes"][1]["change"]["after"]["app_id"] = json!("unrelated");
    assert!(!proven(&fixture, domain, branches));
    assert!(!proven(&fixture, webhook, "branch_name"));
    assert!(label(super::AMPLIFY_DOMAIN, "certificate_verification_dns_record").is_none());
    assert!(label(super::AMPLIFY_WEBHOOK, "url").is_none());
}

#[test]
fn appconfig_profiles_and_versions_require_the_right_application_and_source() {
    let uri = "s3://config-artifacts/settings.json";
    let mut fixture = plan(&[
        (
            "aws_appconfig_application.site",
            json!({"id":"app123"}),
            json!({}),
        ),
        (
            "aws_s3_object.config",
            json!({"key":"settings.json","bucket":"config-artifacts"}),
            json!({}),
        ),
        (
            "aws_appconfig_configuration_profile.hosted",
            json!({"application_id":"app123","configuration_profile_id":"p123","location_uri":"hosted"}),
            json!({"application_id":{"references":["aws_appconfig_application.site.id"]}}),
        ),
        (
            "aws_appconfig_configuration_profile.external",
            json!({"application_id":"app123","configuration_profile_id":"p456","location_uri":uri}),
            json!({"application_id":{"references":["aws_appconfig_application.site.id"]},
                "location_uri":{"references":["aws_s3_object.config.key","aws_s3_object.config"]}}),
        ),
        (
            "aws_appconfig_hosted_configuration_version.current",
            json!({"application_id":"app123","configuration_profile_id":"p123","version_number":1,"content":"private"}),
            json!({"application_id":{"references":["aws_appconfig_application.site.id"]},
                "configuration_profile_id":{"references":["aws_appconfig_configuration_profile.hosted.configuration_profile_id"]}}),
        ),
    ]);
    let external = "aws_appconfig_configuration_profile.external";
    let version = "aws_appconfig_hosted_configuration_version.current";
    let check = |fixture: &Value, address: &str, path: &str| {
        verified(
            &graph(fixture),
            address,
            path,
            label(address.split('.').next().unwrap(), path),
        )
    };
    assert!(check(&fixture, external, "location_uri"));
    assert!(check(&fixture, version, "configuration_profile_id"));
    let result = graph(&fixture);
    assert!(
        result
            .nodes
            .iter()
            .find(|n| n.address == version)
            .unwrap()
            .member_owner
            .is_some()
    );
    fixture["resource_changes"][1]["change"]["after"]["bucket"] = json!("unrelated");
    fixture["resource_changes"][2]["change"]["after"]["application_id"] = json!("other");
    assert!(!check(&fixture, external, "location_uri"));
    assert!(!check(&fixture, version, "configuration_profile_id"));
    assert!(label(super::CONFIG_VERSION, "content").is_none());
}

#[test]
fn appconfig_monitor_set_keeps_role_and_alarm_references_separate() {
    let role = "arn:aws:iam::123456789012:role/Monitor";
    let first = "arn:aws:cloudwatch:us-east-1:123456789012:alarm:first";
    let second = "arn:aws:cloudwatch:us-east-1:123456789012:alarm:second";
    let fixture = plan(&[
        ("aws_iam_role.monitor", json!({"arn":role}), json!({})),
        (
            "aws_cloudwatch_metric_alarm.first",
            json!({"arn":first}),
            json!({}),
        ),
        (
            "aws_cloudwatch_composite_alarm.second",
            json!({"arn":second}),
            json!({}),
        ),
        (
            "aws_appconfig_environment.dev",
            json!({"monitor":[{"alarm_arn":second},{"alarm_arn":first,"alarm_role_arn":role}]}),
            json!({"monitor":[
                {"alarm_arn":{"references":["aws_cloudwatch_metric_alarm.first.arn"]},"alarm_role_arn":{"references":["aws_iam_role.monitor.arn"]}},
                {"alarm_arn":{"references":["aws_cloudwatch_composite_alarm.second.arn"]}}
            ]}),
        ),
    ]);
    for path in ["monitor.alarm_arn", "monitor.alarm_role_arn"] {
        assert!(
            verified(
                &graph(&fixture),
                "aws_appconfig_environment.dev",
                path,
                label(super::CONFIG_ENV, path)
            ),
            "{path}"
        );
    }
}

#[test]
fn appconfig_deployment_limits_versions_and_ownership_to_the_configured_context() {
    let mut fixture = plan(&[
        (
            "aws_appconfig_application.site",
            json!({"id":"app123"}),
            json!({}),
        ),
        (
            "aws_appconfig_environment.dev",
            json!({"application_id":"app123","environment_id":"env1"}),
            json!({"application_id":{"references":["aws_appconfig_application.site.id"]}}),
        ),
        (
            "aws_appconfig_configuration_profile.settings",
            json!({"application_id":"app123","configuration_profile_id":"p123","location_uri":"hosted"}),
            json!({"application_id":{"references":["aws_appconfig_application.site.id"]}}),
        ),
        (
            "aws_appconfig_hosted_configuration_version.initial",
            json!({"application_id":"app123","configuration_profile_id":"p123","version_number":2,"version_label":"stable"}),
            json!({"application_id":{"references":["aws_appconfig_application.site.id"]},
                "configuration_profile_id":{"references":["aws_appconfig_configuration_profile.settings.configuration_profile_id"]}}),
        ),
        (
            "aws_appconfig_deployment_strategy.gradual",
            json!({"id":"st123"}),
            json!({}),
        ),
        (
            "aws_appconfig_deployment.release",
            json!({"application_id":"app123","environment_id":"env1","configuration_profile_id":"p123","configuration_version":"2","deployment_strategy_id":"st123"}),
            json!({
                "application_id":{"references":["aws_appconfig_application.site.id"]},
                "environment_id":{"references":["aws_appconfig_environment.dev.environment_id"]},
                "configuration_profile_id":{"references":["aws_appconfig_configuration_profile.settings.configuration_profile_id"]},
                "configuration_version":{"references":["aws_appconfig_hosted_configuration_version.initial.version_number"]},
                "deployment_strategy_id":{"references":["aws_appconfig_deployment_strategy.gradual.id"]}
            }),
        ),
    ]);
    let address = "aws_appconfig_deployment.release";
    for path in [
        "application_id",
        "environment_id",
        "configuration_profile_id",
        "configuration_version",
        "deployment_strategy_id",
    ] {
        assert!(
            verified(
                &graph(&fixture),
                address,
                path,
                label(super::CONFIG_DEPLOYMENT, path)
            ),
            "{path}"
        );
    }
    assert_eq!(
        graph(&fixture)
            .nodes
            .iter()
            .find(|n| n.address == address)
            .unwrap()
            .member_owner
            .as_ref()
            .unwrap()
            .attribute,
        "environment_id"
    );
    for owner in [
        ("after", json!("unrelated-app")),
        ("after_unknown", json!(true)),
        ("after_sensitive", json!(true)),
    ] {
        let mut different_profile = fixture.clone();
        different_profile["resource_changes"][2]["change"][owner.0]["application_id"] = owner.1;
        assert!(!verified(
            &graph(&different_profile),
            address,
            "configuration_version",
            label(super::CONFIG_DEPLOYMENT, "configuration_version")
        ));
    }
    fixture["resource_changes"][3]["change"]["after_unknown"] = json!({"version_number":true});
    fixture["resource_changes"][1]["change"]["after"]["application_id"] = json!("unrelated-app");
    let result = graph(&fixture);
    assert!(!verified(
        &result,
        address,
        "configuration_version",
        label(super::CONFIG_DEPLOYMENT, "configuration_version")
    ));
    assert!(
        result
            .nodes
            .iter()
            .find(|n| n.address == address)
            .unwrap()
            .member_owner
            .is_none()
    );
    assert!(label(super::CONFIG_DEPLOYMENT, "state").is_none());
    assert!(label(super::CONFIG_DEPLOYMENT, "kms_key_arn").is_none());
}

#[test]
fn appconfig_extension_matches_unordered_actions_and_typed_targets() {
    let lambda = "arn:aws:lambda:us-east-1:123456789012:function:validation";
    let topic = "arn:aws:sns:us-east-1:123456789012:notifications";
    let role = "arn:aws:iam::123456789012:role/Extension";
    let extension = "arn:aws:appconfig:us-east-1:123456789012:extension/ex1";
    let target = "arn:aws:appconfig:us-east-1:123456789012:application/a1/environment/e1";
    let mut fixture = plan(&[
        (
            "aws_lambda_function.validation",
            json!({"arn":lambda}),
            json!({}),
        ),
        ("aws_sns_topic.notify", json!({"arn":topic}), json!({})),
        ("aws_iam_role.extension", json!({"arn":role}), json!({})),
        (
            "aws_appconfig_environment.dev",
            json!({"arn":target}),
            json!({}),
        ),
        (
            "aws_appconfig_extension.validation",
            json!({"arn":extension,"action_point":[
                {"point":"ON_DEPLOYMENT_START","action":[{"uri":topic},{"uri":lambda,"role_arn":role}]},
                {"point":"PRE_START_DEPLOYMENT","action":[{"uri":lambda,"role_arn":role}]}
            ]}),
            json!({"action_point":[
                {"action":[{"uri":{"references":["aws_lambda_function.validation.arn"]},"role_arn":{"references":["aws_iam_role.extension.arn"]}}]},
                {"action":[{"uri":{"references":["aws_lambda_function.validation.arn"]},"role_arn":{"references":["aws_iam_role.extension.arn"]}},{"uri":{"references":["aws_sns_topic.notify.arn"]}}]}
            ]}),
        ),
        (
            "aws_appconfig_extension_association.dev",
            json!({"extension_arn":extension,"resource_arn":target}),
            json!({
                "extension_arn":{"references":["aws_appconfig_extension.validation.arn"]},
                "resource_arn":{"references":["aws_appconfig_environment.dev.arn"]}
            }),
        ),
    ]);
    let paths = [
        (
            "aws_appconfig_extension.validation",
            "action_point.action.uri",
        ),
        (
            "aws_appconfig_extension.validation",
            "action_point.action.role_arn",
        ),
        ("aws_appconfig_extension_association.dev", "extension_arn"),
        ("aws_appconfig_extension_association.dev", "resource_arn"),
    ];
    let check = |fixture: &Value, address: &str, path: &str| {
        verified(
            &graph(fixture),
            address,
            path,
            label(address.split('.').next().unwrap(), path),
        )
    };
    for (address, path) in paths {
        assert!(check(&fixture, address, path), "{address}/{path}");
    }
    fixture["resource_changes"][4]["change"]["after"]["action_point"][0]["point"] =
        json!("PRE_START_DEPLOYMENT");
    assert!(!check(&fixture, paths[0].0, paths[0].1));
    let other = "arn:aws:appconfig:us-east-1:123456789012:application/a1/configurationprofile/p1";
    fixture["resource_changes"][3]["change"]["after"]["arn"] = json!(other);
    fixture["resource_changes"][5]["change"]["after"]["resource_arn"] = json!(other);
    assert!(!check(&fixture, paths[3].0, paths[3].1));
}

#[test]
fn appfabric_authorization_connections_use_the_qualified_bundle() {
    let bundle = "arn:aws:appfabric:us-east-1:123456789012:appbundle/b1";
    let authorization = format!("{bundle}/appauthorization/01010101-0101-0101-0101-010101010101");
    let key = "arn:aws:kms:us-east-1:123456789012:key/01010101-0101-0101-0101-010101010101";
    let mut fixture = plan(&[
        ("aws_kms_key.bundle", json!({"arn":key}), json!({})),
        (
            "aws_appfabric_app_bundle.main",
            json!({"arn":bundle,"customer_managed_key_arn":key}),
            json!({
                "customer_managed_key_arn":{"references":["aws_kms_key.bundle.arn"]}
            }),
        ),
        (
            "aws_appfabric_app_authorization.saas",
            json!({"arn":authorization,"app_bundle_arn":bundle,"app":"SLACK","tenant":[{"tenant_identifier":"tenant"}]}),
            json!({
                "app_bundle_arn":{"references":["aws_appfabric_app_bundle.main.arn"]}
            }),
        ),
        (
            "aws_appfabric_app_authorization_connection.saas",
            json!({"app_bundle_arn":bundle,"app_authorization_arn":authorization}),
            json!({
                "app_bundle_arn":{"references":["aws_appfabric_app_bundle.main.arn"]},
                "app_authorization_arn":{"references":["aws_appfabric_app_authorization.saas.arn"]}
            }),
        ),
    ]);
    let connection = "aws_appfabric_app_authorization_connection.saas";
    let path = "app_authorization_arn";
    let check = |fixture: &Value| {
        verified(
            &graph(fixture),
            connection,
            path,
            label(super::FABRIC_CONNECTION, path),
        )
    };
    assert!(check(&fixture));
    fixture["resource_changes"][3]["change"]["after"][path] =
        json!("01010101-0101-0101-0101-010101010101");
    assert!(check(&fixture));
    fixture["resource_changes"][2]["change"]["after"]["app_bundle_arn"] = json!("unrelated");
    assert!(!check(&fixture));
    assert!(
        graph(&fixture)
            .nodes
            .iter()
            .find(|n| n.address == connection)
            .unwrap()
            .member_owner
            .is_none()
    );
    assert!(label(super::FABRIC_CONNECTION, "tenant").is_none());
}

#[test]
fn appfabric_qualifies_ingestion_destination_and_implicit_authorization() {
    let bundle = "arn:aws:appfabric:us-east-1:123456789012:appbundle/b1";
    let auth = format!("{bundle}/appauthorization/auth1");
    let ingestion = format!("{bundle}/ingestion/in1");
    let mut fixture = plan(&[
        (
            "aws_appfabric_app_bundle.main",
            json!({"arn":bundle}),
            json!({}),
        ),
        (
            "aws_appfabric_app_authorization.saas",
            json!({"arn":auth,"app_bundle_arn":bundle,"app":"SLACK","tenant":[{"tenant_identifier":"tenant"}]}),
            json!({
                "app_bundle_arn":{"references":["aws_appfabric_app_bundle.main.arn"]}
            }),
        ),
        (
            "aws_appfabric_ingestion.saas",
            json!({"arn":ingestion,"app_bundle_arn":bundle,"app":"SLACK","tenant_id":"tenant","ingestion_type":"auditLog"}),
            json!({
                "app_bundle_arn":{"references":["aws_appfabric_app_bundle.main.arn"]}
            }),
        ),
        (
            "aws_s3_bucket.logs",
            json!({"bucket":"logging-bucket"}),
            json!({}),
        ),
        (
            "aws_appfabric_ingestion_destination.logs",
            json!({"app_bundle_arn":bundle,"ingestion_arn":ingestion,
                "destination_configuration":[{"audit_log":[{"destination":[{"s3_bucket":[{"bucket_name":"logging-bucket"}]}]}]}]
            }),
            json!({
                "app_bundle_arn":{"references":["aws_appfabric_app_bundle.main.arn"]},
                "ingestion_arn":{"references":["aws_appfabric_ingestion.saas.arn"]},
                "destination_configuration":[{"audit_log":[{"destination":[{"s3_bucket":[{"bucket_name":{"references":["aws_s3_bucket.logs.bucket"]}}]}]}]}]
            }),
        ),
    ]);
    let ingestion = "aws_appfabric_ingestion.saas";
    let destination = "aws_appfabric_ingestion_destination.logs";
    let check = |fixture: &Value, address: &str, path: &str| {
        verified(
            &graph(fixture),
            address,
            path,
            label(address.split('.').next().unwrap(), path),
        )
    };
    assert!(check(&fixture, ingestion, "authorization"));
    assert!(check(&fixture, destination, "ingestion_arn"));
    assert!(check(&fixture, destination, super::FABRIC_S3));
    for key in ["resource_changes", "configuration"] {
        let rows = if key == "configuration" {
            &mut fixture[key]["root_module"]["resources"]
        } else {
            &mut fixture[key]
        };
        let mut duplicate = rows[1].clone();
        duplicate["name"] = json!("other");
        duplicate["address"] = json!("aws_appfabric_app_authorization.other");
        if key == "resource_changes" {
            duplicate["change"]["after"]["arn"] = json!(format!("{bundle}/appauthorization/auth2"));
        }
        rows.as_array_mut().unwrap().push(duplicate);
    }
    assert!(!check(&fixture, ingestion, "authorization"));
    fixture["resource_changes"][5]["change"]["after_unknown"] =
        json!({"tenant":[{"tenant_identifier":true}]});
    assert!(!check(&fixture, ingestion, "authorization"));
    fixture["resource_changes"][5]["change"]["after"]["app"] = json!("DIFFERENT");
    assert!(check(&fixture, ingestion, "authorization"));
    fixture["resource_changes"][4]["change"]["after"]["app_bundle_arn"] = json!("unrelated");
    assert!(!check(&fixture, destination, "ingestion_arn"));
    assert!(
        graph(&fixture)
            .nodes
            .iter()
            .find(|n| n.address == destination)
            .unwrap()
            .member_owner
            .is_none()
    );
}

#[test]
fn appflow_matches_profiles_and_buckets_under_the_selected_connector() {
    let key = "arn:aws:kms:us-east-1:123456789012:key/appflow";
    let role = "arn:aws:iam::123456789012:role/Redshift";
    let mut fixture = plan(&[
        ("aws_kms_key.flow", json!({"arn":key}), json!({})),
        ("aws_iam_role.redshift", json!({"arn":role}), json!({})),
        (
            "aws_s3_bucket.output",
            json!({"bucket":"target-bucket"}),
            json!({}),
        ),
        (
            "aws_appflow_connector_profile.crm",
            json!({"name":"crm","connector_type":"Salesforce"}),
            json!({}),
        ),
        (
            "aws_appflow_connector_profile.warehouse",
            json!({"name":"warehouse","connector_type":"Redshift","connector_profile_config":[{"connector_profile_properties":[{"redshift":[{"role_arn":role,"data_api_role_arn":role}]}]}]}),
            json!({
                "connector_profile_config":[{"connector_profile_properties":[{"redshift":[{"role_arn":{"references":["aws_iam_role.redshift.arn"]},"data_api_role_arn":{"references":["aws_iam_role.redshift.arn"]}}]}]}]
            }),
        ),
        (
            "aws_appflow_flow.pipeline",
            json!({
                "kms_arn":key,"source_flow_config":[{"connector_type":"Salesforce","connector_profile_name":"crm"}],
                "destination_flow_config":[{"connector_type":"S3","destination_connector_properties":[{"s3":[{"bucket_name":"target-bucket"}]}]},
                    {"connector_type":"Redshift","connector_profile_name":"warehouse"}]
            }),
            json!({
                "kms_arn":{"references":["aws_kms_key.flow.arn"]},
                "source_flow_config":[{"connector_profile_name":{"references":["aws_appflow_connector_profile.crm.name"]}}],
                "destination_flow_config":[{"destination_connector_properties":[{"s3":[{"bucket_name":{"references":["aws_s3_bucket.output.bucket"]}}]}]},
                    {"connector_profile_name":{"references":["aws_appflow_connector_profile.warehouse.name"]}}]
            }),
        ),
    ]);
    let flow = "aws_appflow_flow.pipeline";
    let profile = "aws_appflow_connector_profile.warehouse";
    let check = |fixture: &Value, address: &str, path: &str| {
        verified(
            &graph(fixture),
            address,
            path,
            label(address.split('.').next().unwrap(), path),
        )
    };
    for path in [
        "kms_arn",
        "source_flow_config.connector_profile_name",
        "destination_flow_config.connector_profile_name",
        "destination_flow_config.destination_connector_properties.s3.bucket_name",
    ] {
        assert!(check(&fixture, flow, path), "{path}");
    }
    assert!(check(
        &fixture,
        profile,
        "connector_profile_config.connector_profile_properties.redshift.role_arn"
    ));
    fixture["resource_changes"][5]["change"]["after"]["destination_flow_config"][0]["connector_type"] =
        json!("Redshift");
    fixture["resource_changes"][4]["change"]["after"]["connector_type"] = json!("Salesforce");
    assert!(!check(
        &fixture,
        flow,
        "destination_flow_config.connector_profile_name"
    ));
    assert!(!check(
        &fixture,
        flow,
        "destination_flow_config.destination_connector_properties.s3.bucket_name"
    ));
    assert!(!check(
        &fixture,
        profile,
        "connector_profile_config.connector_profile_properties.redshift.role_arn"
    ));
    assert!(label(super::FLOW_PROFILE, "credentials_arn").is_none());
}

#[test]
fn appintegrations_checks_the_salesforce_profile_and_event_bus_name() {
    let key = "arn:aws:kms:us-east-1:123456789012:key/integration";
    let mut fixture = plan(&[
        ("aws_kms_key.integration", json!({"arn":key}), json!({})),
        (
            "aws_appflow_connector_profile.crm",
            json!({"name":"crm","connector_type":"Salesforce"}),
            json!({}),
        ),
        (
            "aws_cloudwatch_event_bus.partner",
            json!({"name":"aws.partner/example"}),
            json!({}),
        ),
        (
            "aws_appintegrations_data_integration.crm",
            json!({"source_uri":"Salesforce://AppFlow/crm","kms_key":key}),
            json!({
                "kms_key":{"references":["aws_kms_key.integration.arn"]},
                "source_uri":{"references":["aws_appflow_connector_profile.crm.name"]}
            }),
        ),
        (
            "aws_appintegrations_event_integration.notifications",
            json!({"eventbridge_bus":"aws.partner/example","event_filter":[{"source":"aws.partner/example"}]}),
            json!({
                "eventbridge_bus":{"references":["aws_cloudwatch_event_bus.partner.name"]}
            }),
        ),
    ]);
    let data = "aws_appintegrations_data_integration.crm";
    let event = "aws_appintegrations_event_integration.notifications";
    let check = |fixture: &Value, address: &str, path: &str| {
        verified(
            &graph(fixture),
            address,
            path,
            label(address.split('.').next().unwrap(), path),
        )
    };
    for (address, path) in [
        (data, "kms_key"),
        (data, "source_uri"),
        (event, "eventbridge_bus"),
    ] {
        assert!(check(&fixture, address, path), "{path}");
    }
    fixture["resource_changes"][1]["change"]["after"]["connector_type"] = json!("Zendesk");
    assert!(!check(&fixture, data, "source_uri"));
    fixture["resource_changes"][1]["change"]["after"]["connector_type"] = json!("Salesforce");
    fixture["resource_changes"][3]["change"]["after"]["source_uri"] = json!("unrecognized://crm");
    assert!(!check(&fixture, data, "source_uri"));
    assert!(label(super::EVENT_INTEGRATION, "event_filter.source").is_none());
    assert!(label(super::DATA_INTEGRATION, "schedule_config.object").is_none());
}

#[test]
fn mainframe_deployment_uses_explicit_version_and_compatible_environment() {
    let locator = "s3://definitions-bucket/app.json";
    let key = "arn:aws:kms:us-east-1:123456789012:key/mainframe";
    let mut fixture = plan(&[
        (
            "aws_s3_object.definition",
            json!({"bucket":"definitions-bucket","key":"app.json"}),
            json!({}),
        ),
        ("aws_kms_key.environment", json!({"arn":key}), json!({})),
        (
            "aws_efs_file_system.app",
            json!({"id":"fs-mainframe"}),
            json!({}),
        ),
        (
            "aws_m2_application.main",
            json!({"application_id":"app1","engine_type":"microfocus","current_version":3,"definition":[{"s3_location":locator}]}),
            json!({
                "definition":[{"s3_location":{"references":["aws_s3_object.definition.bucket","aws_s3_object.definition.key"]}}]
            }),
        ),
        (
            "aws_m2_environment.main",
            json!({"environment_id":"env1","engine_type":"microfocus","kms_key_id":key,
                "storage_configuration":[{"efs":[{"file_system_id":"fs-mainframe","mount_point":"/data"}]}],
                "publicly_accessible":true,"load_balancer_arn":"arn:aws:elasticloadbalancing:us-east-1:123456789012:loadbalancer/returned"
            }),
            json!({
                "kms_key_id":{"references":["aws_kms_key.environment.arn"]},
                "storage_configuration":[{"efs":[{"file_system_id":{"references":["aws_efs_file_system.app.id"]}}]}]
            }),
        ),
        (
            "aws_m2_deployment.main",
            json!({"application_id":"app1","environment_id":"env1","application_version":3,"start":true}),
            json!({
                "application_id":{"references":["aws_m2_application.main.application_id"]},
                "environment_id":{"references":["aws_m2_environment.main.environment_id"]},
                "application_version":{"references":["aws_m2_application.main.current_version"]}
            }),
        ),
    ]);
    let deployment = "aws_m2_deployment.main";
    let check = |fixture: &Value, address: &str, path: &str| {
        verified(
            &graph(fixture),
            address,
            path,
            label(address.split('.').next().unwrap(), path),
        )
    };
    for (address, path) in [
        ("aws_m2_application.main", "definition.s3_location"),
        (
            "aws_m2_environment.main",
            "storage_configuration.efs.file_system_id",
        ),
        ("aws_m2_environment.main", "kms_key_id"),
        (deployment, "application_id"),
        (deployment, "environment_id"),
        (deployment, "application_version"),
    ] {
        assert!(check(&fixture, address, path), "{path}");
    }
    fixture["resource_changes"][4]["change"]["after"]["engine_type"] = json!("bluage");
    assert!(!check(&fixture, deployment, "environment_id"));
    fixture["configuration"]["root_module"]["resources"][5]["expressions"]["application_version"] =
        json!({"constant_value":3});
    assert!(!check(&fixture, deployment, "application_version"));
    assert!(label(super::MAINFRAME_ENV, "load_balancer_arn").is_none());
    assert!(label(super::MAINFRAME_ENV, "publicly_accessible").is_none());
    assert!(label(super::MAINFRAME_DEPLOYMENT, "start").is_none());
}

#[test]
fn mainframe_network_requires_complete_subnets_and_groups_in_one_vpc() {
    let mut fixture = plan(&[
        (
            "aws_vpc.main",
            json!({"id":"vpc-main","cidr_block":"10.0.0.0/16"}),
            json!({}),
        ),
        (
            "aws_subnet.a",
            json!({"id":"subnet-a","vpc_id":"vpc-main","cidr_block":"10.0.1.0/24"}),
            json!({"vpc_id":{"references":["aws_vpc.main.id"]}}),
        ),
        (
            "aws_subnet.b",
            json!({"id":"subnet-b","vpc_id":"vpc-main","cidr_block":"10.0.2.0/24"}),
            json!({"vpc_id":{"references":["aws_vpc.main.id"]}}),
        ),
        (
            "aws_security_group.main",
            json!({"id":"sg-main","vpc_id":"vpc-main"}),
            json!({"vpc_id":{"references":["aws_vpc.main.id"]}}),
        ),
        (
            "aws_m2_environment.main",
            json!({"subnet_ids":["subnet-b","subnet-a"],"security_group_ids":["sg-main"]}),
            json!({
                "subnet_ids":{"references":["aws_subnet.a.id","aws_subnet.b.id"]},
                "security_group_ids":{"references":["aws_security_group.main.id"]}
            }),
        ),
    ]);
    let vpc = |fixture: &Value| {
        let graph = graph(fixture);
        graph.edges.iter().any(|edge| {
            edge.kind == crate::model::EdgeKind::Containment
                && graph.nodes[edge.from].address == "aws_vpc.main"
                && graph.nodes[edge.to].address == "aws_m2_environment.main"
        })
    };
    let placement_is_logical = |fixture: &Value| {
        let rendered = crate::generate(&fixture.to_string(), &crate::Options::default()).unwrap();
        let svg = roxmltree::Document::parse(&rendered.content).unwrap();
        svg.descendants()
            .find(|element| {
                element.attribute("data-terraform-address") == Some("aws_m2_environment.main")
            })
            .unwrap()
            .attribute("data-placement")
            == Some("logical")
    };
    assert!(vpc(&fixture));
    assert!(!placement_is_logical(&fixture));
    fixture["resource_changes"][4]["change"]["after_sensitive"] =
        json!({"security_group_ids":true});
    assert!(!vpc(&fixture));
    assert!(placement_is_logical(&fixture));
    fixture["resource_changes"][4]["change"]["after_sensitive"] = Value::Null;
    fixture["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("vpc-unrelated");
    assert!(!vpc(&fixture));
}
