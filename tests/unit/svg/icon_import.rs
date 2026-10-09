#[path = "../../../build/icon.rs"]
mod importer;

#[test]
fn namespaces_geometry_gradients_clips_masks_and_local_uses() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 64 64"><defs><linearGradient id="paint"><stop offset="0" stop-color="#fff"/></linearGradient><clipPath id="clip"><rect width="10" height="10"/></clipPath><mask id="mask"><circle r="4"/></mask></defs><g id="shape" fill="url(#paint)" clip-path="url(#clip)" mask="url(#mask)"><path d="M0 0L1 1"/><path d="M1 1L2 2"/></g><use xlink:href="#shape"/></svg>"##;
    let one = importer::symbol(source, "one").unwrap();
    let two = importer::symbol(source, "two").unwrap();
    assert_eq!(one.matches("<path ").count(), 2);
    assert!(one.contains("fill=\"url(#one-local-7061696e74)\""));
    assert!(one.contains("href=\"#one-local-7368617065\""));
    let combined = format!("<svg>{one}{two}</svg>");
    let doc = roxmltree::Document::parse(&combined).unwrap();
    let ids: Vec<_> = doc
        .descendants()
        .filter_map(|n| n.attribute("id"))
        .collect();
    assert_eq!(
        ids.len(),
        ids.iter().collect::<std::collections::BTreeSet<_>>().len()
    );
}

#[test]
fn rejects_executable_external_and_ambiguous_content() {
    for content in [
        "<script/>",
        "<image href=\"https://example.com/x\"/>",
        "<path onclick=\"alert(1)\"/>",
        "<style>path{fill:red}</style>",
        "<use href=\"data:image/svg+xml,x\"/>",
        "<path fill=\"url(https://example.com/x)\"/>",
        "<path fill=\"URL(#x)\"/>",
        "<use href=\"#missing\"/>",
        "<g id=\"x\"/><g id=\"x\"/>",
        "<animate/>",
        "<?test hi?>",
    ] {
        let source = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\">{content}</svg>"
        );
        assert!(importer::symbol(&source, "test").is_err(), "{content}");
    }
    assert!(importer::symbol("<!DOCTYPE svg [<!ENTITY x 'x'>]><svg/>", "test").is_err());
}
