use crate::{Options, OutputFormat, generate};

#[test]
fn category_badges_share_colors_across_presentations_and_preserve_actions() {
    let input = serde_json::json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_instance.compute","type":"aws_instance","change":{"actions":["create"]}},
        {"address":"aws_s3_bucket.storage","type":"aws_s3_bucket","change":{"actions":["delete"]}},
        {"address":"aws_vpc.network","type":"aws_vpc","change":{"actions":["update"]}},
        {"address":"aws_vpc_endpoint_route_table_association.logical","type":"aws_vpc_endpoint_route_table_association"},
        {"address":"aws_iam_role.neutral","type":"aws_iam_role"},
        {"address":"terraform_data.neutral","type":"terraform_data"},
        {"address":"aws_instance.foreign","type":"aws_instance","provider_name":"example.com/other/aws"},
        {"address":"aws_unknown.main","type":"aws_unknown"}
    ]}).to_string();
    for format in [OutputFormat::Svg, OutputFormat::Html] {
        let output = generate(
            &input,
            &Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        let content = &output.content;
        let svg = &content[content.find("<svg").unwrap()..content.rfind("</svg>").unwrap() + 6];
        let doc = roxmltree::Document::parse(svg).unwrap();
        for (address, category, background, foreground) in [
            ("aws_instance.compute", "compute", "#ffedd5", "#7c2d12"),
            ("aws_s3_bucket.storage", "storage", "#dcfce7", "#14532d"),
            ("aws_vpc.network", "network", "#dbeafe", "#1e3a8a"),
            (
                "aws_vpc_endpoint_route_table_association.logical",
                "network",
                "#dbeafe",
                "#1e3a8a",
            ),
            ("aws_iam_role.neutral", "iam", "#fee2e2", "#7f1d1d"),
            ("terraform_data.neutral", "neutral", "#e2e8f0", "#0f172a"),
        ] {
            let row = doc
                .descendants()
                .find(|n| n.attribute("data-terraform-address") == Some(address))
                .unwrap();
            let badge = row
                .children()
                .find(|n| n.attribute("data-service-badge").is_some())
                .unwrap();
            assert_eq!(badge.attribute("data-service-category"), Some(category));
            assert_eq!(badge.attribute("fill"), Some(background));
            for text in row
                .children()
                .filter(|n| n.attribute("data-service-name").is_some())
            {
                assert_eq!(text.attribute("fill"), Some(foreground));
            }
            assert!(!row.children().any(|n| n.has_tag_name("rect")
                && n.attribute("data-service-badge").is_none()
                && n.attribute("fill") == Some(background)));
        }
        for address in ["aws_instance.foreign", "aws_unknown.main"] {
            let row = doc
                .descendants()
                .find(|n| n.attribute("data-terraform-address") == Some(address))
                .unwrap();
            assert!(
                !row.descendants()
                    .any(|n| n.attribute("data-service-category").is_some())
            );
        }
    }
}

#[test]
fn category_text_contrast_remains_readable_without_color_distinction() {
    use crate::services::Category;
    fn luminance(color: &str) -> f64 {
        let channels: Vec<_> = [1, 3, 5]
            .into_iter()
            .map(|start| {
                let value =
                    u8::from_str_radix(&color[start..start + 2], 16).unwrap() as f64 / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            })
            .collect();
        channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
    }
    for category in [
        Category::Compute,
        Category::Storage,
        Category::Network,
        Category::Iam,
        Category::Neutral,
    ] {
        let (_, background, foreground) = category.palette();
        assert!(
            (luminance(background) + 0.05) / (luminance(foreground) + 0.05) >= 4.5,
            "{category:?}"
        );
    }
}
