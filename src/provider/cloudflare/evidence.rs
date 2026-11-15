use crate::model::CloudflareFailure as Failure;
use serde_json::Value;

pub(super) type Result<T> = std::result::Result<T, Failure>;

#[derive(Clone, Copy)]
pub(crate) struct Evidence<'a> {
    pub value: &'a Value,
    pub mask: &'a Value,
    pub unknown: &'a Value,
    pub sensitive: &'a Value,
}

impl<'a> Evidence<'a> {
    fn failure(self) -> Failure {
        fn marked(mask: &Value) -> bool {
            match mask {
                Value::Bool(true) => true,
                Value::Object(fields) => fields.values().any(marked),
                Value::Array(values) => values.iter().any(marked),
                _ => false,
            }
        }
        if marked(self.sensitive) {
            Failure::SensitiveValue
        } else if marked(self.unknown) {
            Failure::UnknownValue
        } else {
            Failure::InvalidValue
        }
    }

    pub(super) fn clear(self) -> Result<()> {
        super::proof::unmasked(self.mask)
            .then_some(())
            .ok_or_else(|| self.failure())
    }

    pub(super) fn scalar(self) -> Result<()> {
        matches!(self.mask, Value::Null | Value::Bool(false))
            .then_some(())
            .ok_or_else(|| self.failure())
    }

    pub(super) fn at(self, path: &str) -> Result<Self> {
        fn mask_at<'a>(mut mask: &'a Value, path: &str) -> &'a Value {
            for field in path.split('.') {
                if mask == &Value::Bool(true) {
                    return mask;
                }
                mask = &mask[field];
            }
            mask
        }
        path.split('.').try_fold(self, Self::child).map_err(|_| {
            Self {
                unknown: mask_at(self.unknown, path),
                sensitive: mask_at(self.sensitive, path),
                ..self
            }
            .failure()
        })
    }

    pub(super) fn child(self, field: &str) -> Result<Self> {
        if !self.value.is_object()
            || !matches!(
                self.mask,
                Value::Null | Value::Bool(false) | Value::Object(_)
            )
        {
            return Err(self.failure());
        }
        Ok(Self {
            value: &self.value[field],
            mask: &self.mask[field],
            unknown: &self.unknown[field],
            sensitive: &self.sensitive[field],
        })
    }

    pub(super) fn text(self, path: &str) -> Result<&'a str> {
        let field = self.at(path)?;
        field.scalar()?;
        field
            .value
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or(Failure::InvalidValue)
    }

    pub(super) fn id(self, path: &str) -> Result<&'a str> {
        self.text(path).and_then(|id| {
            (id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()))
                .then_some(id)
                .ok_or(Failure::InvalidValue)
        })
    }

    pub(super) fn rows(self) -> Result<Vec<Self>> {
        if !matches!(
            self.mask,
            Value::Null | Value::Bool(false) | Value::Array(_)
        ) {
            return Err(self.failure());
        }
        let values = self.value.as_array().ok_or(Failure::InvalidValue)?;
        if self
            .mask
            .as_array()
            .is_some_and(|m| m.len() != values.len())
        {
            return Err(Failure::IncompleteCollection);
        }
        Ok(values
            .iter()
            .enumerate()
            .map(|(index, value)| Self {
                value,
                mask: &self.mask[index],
                unknown: &self.unknown[index],
                sensitive: &self.sensitive[index],
            })
            .collect())
    }

    pub(super) fn list(self) -> Result<Vec<&'a str>> {
        let values = collect(self.rows()?.into_iter().map(|row| {
            row.scalar()?;
            row.value
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or(Failure::InvalidValue)
        }))?;
        if values
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != values.len()
        {
            return Err(Failure::IncompleteCollection);
        }
        Ok(values)
    }

    pub(super) fn map(self) -> Result<Vec<(&'a str, Self)>> {
        if !matches!(
            self.mask,
            Value::Null | Value::Bool(false) | Value::Object(_)
        ) {
            return Err(self.failure());
        }
        let values = self.value.as_object().ok_or(Failure::InvalidValue)?;
        if self
            .mask
            .as_object()
            .is_some_and(|m| m.keys().any(|k| !values.contains_key(k)))
        {
            return Err(Failure::IncompleteCollection);
        }
        values
            .keys()
            .map(|key| self.child(key).map(|value| (key.as_str(), value)))
            .collect()
    }
}

pub(super) fn collect<T>(results: impl IntoIterator<Item = Result<T>>) -> Result<Vec<T>> {
    let mut values = Vec::new();
    let mut failure = None;
    for result in results {
        match result {
            Ok(value) => values.push(value),
            Err(reason) => {
                failure = Some(failure.map_or(reason, |previous: Failure| previous.min(reason)))
            }
        }
    }
    failure.map_or(Ok(values), Err)
}
