use std::collections::BTreeMap;
use std::fmt::Write;

const SVG: &str = "http://www.w3.org/2000/svg";
const XLINK: &str = "http://www.w3.org/1999/xlink";

/// Build-time allowlist: executable elements, CSS and external references fail the build.
pub fn symbol(source: &str, prefix: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(source).map_err(|e| e.to_string())?;
    let root = doc.root_element();
    if !root.has_tag_name((SVG, "svg")) {
        return Err("expected SVG root".into());
    }
    let mut ids = BTreeMap::new();
    for node in doc.descendants() {
        if node.is_pi() {
            return Err("processing instructions are not allowed".into());
        }
        if let Some(id) = node.attribute("id") {
            let mut encoded = format!("{prefix}-local-");
            for byte in id.bytes() {
                write!(encoded, "{byte:02x}").unwrap();
            }
            if ids.insert(id, encoded).is_some() {
                return Err("duplicate local ID".into());
            }
        }
    }
    let view_box = root.attribute("viewBox").ok_or("missing viewBox")?;
    let numbers: Vec<f64> = view_box
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|v| !v.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|_| "invalid viewBox")?;
    if numbers.len() != 4
        || numbers.iter().any(|n| !n.is_finite())
        || numbers[2] <= 0.0
        || numbers[3] <= 0.0
    {
        return Err("invalid viewBox".into());
    }
    let mut out = format!(
        "<symbol id=\"{}\" viewBox=\"{}\">",
        escape(prefix),
        escape(view_box)
    );
    emit(root, &ids, &mut out, true)?;
    out.push_str("</symbol>");
    Ok(out)
}

fn emit(
    node: roxmltree::Node<'_, '_>,
    ids: &BTreeMap<&str, String>,
    out: &mut String,
    root: bool,
) -> Result<(), String> {
    if node.is_comment() {
        return Ok(());
    } // Source comments have no rendering semantics.
    if node.is_text() {
        out.push_str(&escape(node.text().unwrap_or_default()));
        return Ok(());
    }
    let tag = node.tag_name();
    if tag.namespace() != Some(SVG)
        || !matches!(
            tag.name(),
            "svg"
                | "g"
                | "defs"
                | "path"
                | "rect"
                | "circle"
                | "ellipse"
                | "line"
                | "polyline"
                | "polygon"
                | "linearGradient"
                | "radialGradient"
                | "stop"
                | "clipPath"
                | "mask"
                | "use"
                | "title"
                | "desc"
        )
    {
        return Err(format!("unsupported element: {}", tag.name()));
    }
    let name = if root { "g" } else { tag.name() };
    write!(out, "<{name}").unwrap();
    for attr in node.attributes() {
        let key = attr.name();
        if attr.namespace().is_some() && !(attr.namespace() == Some(XLINK) && key == "href") {
            return Err("unsupported attribute namespace".into());
        }
        if !matches!(
            key,
            "id" | "viewBox"
                | "version"
                | "width"
                | "height"
                | "x"
                | "y"
                | "x1"
                | "x2"
                | "y1"
                | "y2"
                | "cx"
                | "cy"
                | "r"
                | "rx"
                | "ry"
                | "fx"
                | "fy"
                | "d"
                | "points"
                | "transform"
                | "fill"
                | "fill-rule"
                | "fill-opacity"
                | "stroke"
                | "stroke-width"
                | "stroke-opacity"
                | "stroke-linecap"
                | "stroke-linejoin"
                | "stroke-miterlimit"
                | "stroke-dasharray"
                | "stroke-dashoffset"
                | "opacity"
                | "clip-path"
                | "clip-rule"
                | "mask"
                | "maskUnits"
                | "maskContentUnits"
                | "clipPathUnits"
                | "gradientUnits"
                | "gradientTransform"
                | "spreadMethod"
                | "offset"
                | "stop-color"
                | "stop-opacity"
                | "href"
                | "preserveAspectRatio"
        ) {
            return Err(format!("unsupported attribute: {key}"));
        }
        if root && matches!(key, "viewBox" | "version" | "width" | "height" | "x" | "y") {
            continue;
        }
        let value = if key == "id" {
            ids[attr.value()].clone()
        } else if key == "href" {
            let target = attr
                .value()
                .strip_prefix('#')
                .and_then(|id| ids.get(id))
                .ok_or("href must reference a local ID")?;
            format!("#{target}")
        } else if matches!(key, "fill" | "stroke" | "clip-path" | "mask")
            && attr.value().starts_with("url(#")
            && attr.value().ends_with(')')
        {
            let target = ids
                .get(&attr.value()[5..attr.value().len() - 1])
                .ok_or("unresolved local URL")?;
            format!("url(#{target})")
        } else {
            // No CSS escapes, functions containing URLs, schemes, or alternative URL syntax.
            let value = attr.value();
            if value.contains(['\\', ':', '&']) || value.to_ascii_lowercase().contains("url") {
                return Err("external or unsupported attribute value".into());
            }
            value.to_owned()
        };
        write!(out, " {key}=\"{}\"", escape(&value)).unwrap();
    }
    out.push('>');
    for child in node.children() {
        emit(child, ids, out, false)?;
    }
    write!(out, "</{name}>").unwrap();
    Ok(())
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
