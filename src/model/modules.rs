use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Index {
    Number(u64),
    Key(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Address(Vec<(String, Option<Index>)>);

impl Address {
    pub fn parse(value: &str) -> Result<Self, String> {
        let invalid = || format!("invalid module address: {value:?}");
        let mut rest = value;
        let mut parts = Vec::new();
        loop {
            rest = rest.strip_prefix("module.").ok_or_else(invalid)?;
            let end = rest.find(['.', '[']).unwrap_or(rest.len());
            let name = &rest[..end];
            if name.is_empty()
                || !name.chars().enumerate().all(|(i, c)| {
                    c == '_' || c.is_alphabetic() || (i > 0 && (c.is_numeric() || c == '-'))
                })
            {
                return Err(invalid());
            }
            rest = &rest[end..];
            let index = if let Some(indexed) = rest.strip_prefix('[') {
                let mut stream =
                    serde_json::Deserializer::from_str(indexed).into_iter::<serde_json::Value>();
                let index = match stream.next().transpose().map_err(|_| invalid())? {
                    Some(serde_json::Value::String(key)) => Index::Key(key),
                    Some(serde_json::Value::Number(number)) => {
                        Index::Number(number.as_u64().ok_or_else(invalid)?)
                    }
                    _ => return Err(invalid()),
                };
                rest = indexed[stream.byte_offset()..]
                    .strip_prefix(']')
                    .ok_or_else(invalid)?;
                Some(index)
            } else {
                None
            };
            parts.push((name.to_owned(), index));
            if rest.is_empty() {
                return Ok(Self(parts));
            }
            rest = rest.strip_prefix('.').ok_or_else(invalid)?;
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.0.len() <= other.0.len()
            && self
                .0
                .iter()
                .zip(&other.0)
                .all(|((name, index), (n, i))| name == n && (index.is_none() || index == i))
    }

    pub fn resource(address: &str) -> Result<Self, String> {
        match super::module_of(address) {
            "root" => Ok(Self::default()),
            module => Self::parse(module),
        }
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, (name, index)) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            write!(f, "module.{name}")?;
            match index {
                Some(Index::Number(number)) => write!(f, "[{number}]")?,
                Some(Index::Key(key)) => write!(f, "[{}]", serde_json::to_string(key).unwrap())?,
                None => {}
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory(pub BTreeMap<Address, bool>);

impl Inventory {
    fn insert(&mut self, address: Address, actual: bool) {
        for end in 0..=address.0.len() {
            *self
                .0
                .entry(Address(address.0[..end].to_vec()))
                .or_default() |= actual;
        }
    }

    pub fn collect(
        json: &serde_json::Value,
        nodes: &[super::TerraformEntity],
    ) -> Result<Self, String> {
        fn values(value: &serde_json::Value, result: &mut Inventory) -> Result<(), String> {
            if let Some(address) = value["address"].as_str() {
                result.insert(Address::parse(address)?, true);
            }
            for child in value["child_modules"].as_array().into_iter().flatten() {
                values(child, result)?;
            }
            Ok(())
        }
        fn config(value: &serde_json::Value, parent: &Address, result: &mut Inventory) {
            for (name, call) in value["module_calls"].as_object().into_iter().flatten() {
                let mut child = parent.clone();
                child.0.push((name.clone(), None));
                result.insert(child.clone(), false);
                config(&call["module"], &child, result);
            }
        }
        let mut result = Self::default();
        result.insert(Address::default(), true);
        for node in nodes {
            result.insert(Address::resource(&node.address)?, true);
        }
        values(&json["planned_values"]["root_module"], &mut result)?;
        values(&json["prior_state"]["values"]["root_module"], &mut result)?;
        config(
            &json["configuration"]["root_module"],
            &Address::default(),
            &mut result,
        );
        Ok(result)
    }
}

pub fn selector(value: &str) -> Result<(Option<super::StateId>, Address), String> {
    if value
        .find(':')
        .is_none_or(|colon| value.find('[').is_some_and(|bracket| bracket < colon))
    {
        return Ok((None, Address::parse(value)?));
    }
    let (state, address) = value
        .split_once(':')
        .ok_or_else(|| format!("invalid module selector: {value:?}"))?;
    Ok((Some(super::StateId::new(state)?), Address::parse(address)?))
}

#[cfg(test)]
#[path = "../../tests/unit/model/modules.rs"]
mod tests;
