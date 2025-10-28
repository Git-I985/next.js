use std::fmt;

use anyhow::{Context, Result};
use regex::Regex;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as _, Visitor},
    ser::SerializeTupleStruct,
};
use turbopack_node::route_matcher::{Param, Params, RouteMatcherRef};

/// A regular expression that matches a path, with named capture groups for the
/// dynamic parts of the path.
#[derive(Debug)]
pub struct PathRegex {
    regex: Regex,
    named_params: Vec<NamedParam>,
}

impl PartialEq for PathRegex {
    fn eq(&self, other: &Self) -> bool {
        self.regex.as_str() == other.regex.as_str() && self.named_params == other.named_params
    }
}

impl Eq for PathRegex {}

impl Serialize for PathRegex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ts = serializer.serialize_tuple_struct("PathRegex", 2)?;
        ts.serialize_field(self.regex.as_str())?;
        ts.serialize_field(&self.named_params)?;
        ts.end()
    }
}

impl<'de> Deserialize<'de> for PathRegex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PathRegexVisitor;

        impl<'de> Visitor<'de> for PathRegexVisitor {
            type Value = PathRegex;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a PathRegex tuple struct")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let regex_str: String = seq
                    .next_element()?
                    .ok_or_else(|| A::Error::invalid_length(0, &self))?;
                let named_params: Vec<NamedParam> = seq
                    .next_element()?
                    .ok_or_else(|| A::Error::invalid_length(1, &self))?;

                let regex = Regex::new(&regex_str)
                    .map_err(|e| A::Error::custom(format!("invalid regex: {e}")))?;

                Ok(PathRegex {
                    regex,
                    named_params,
                })
            }
        }

        deserializer.deserialize_tuple_struct("PathRegex", 2, PathRegexVisitor)
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
struct NamedParam {
    name: String,
    kind: NamedParamKind,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
enum NamedParamKind {
    Single,
    Multi,
}

impl std::fmt::Display for PathRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.regex.as_str())
    }
}

impl RouteMatcherRef for PathRegex {
    fn matches(&self, path: &str) -> bool {
        self.regex.is_match(path)
    }

    fn params(&self, path: &str) -> Params {
        Params(self.regex.captures(path).map(|capture| {
            self.named_params
                .iter()
                .enumerate()
                .filter_map(|(idx, param)| {
                    if param.name.is_empty() {
                        return None;
                    }
                    let value = capture.get(idx + 1)?;
                    Some((
                        param.name.as_str().into(),
                        match param.kind {
                            NamedParamKind::Single => Param::Single(value.as_str().into()),
                            NamedParamKind::Multi => Param::Multi(
                                value
                                    .as_str()
                                    .split('/')
                                    .map(|segment| segment.into())
                                    .collect(),
                            ),
                        },
                    ))
                })
                .collect()
        }))
    }
}

/// Builder for [PathRegex].
pub struct PathRegexBuilder {
    regex_str: String,
    named_params: Vec<NamedParam>,
}

impl PathRegexBuilder {
    /// Creates a new [PathRegexBuilder].
    pub fn new() -> Self {
        Self {
            regex_str: "^".to_string(),
            named_params: Default::default(),
        }
    }

    fn include_slash(&self) -> bool {
        self.regex_str.len() > 1
    }

    fn push_str(&mut self, str: &str) {
        self.regex_str.push_str(str);
    }

    /// Pushes an optional catch all segment to the regex.
    pub fn push_optional_catch_all<N, R>(&mut self, name: N, rem: R)
    where
        N: Into<String>,
        R: AsRef<str>,
    {
        self.push_str(if self.include_slash() {
            "(?:/([^?]+))?"
        } else {
            "([^?]+)?"
        });
        self.push_str(&regex::escape(rem.as_ref()));
        self.named_params.push(NamedParam {
            name: name.into(),
            kind: NamedParamKind::Multi,
        });
    }

    /// Pushes a catch all segment to the regex.
    pub fn push_catch_all<N, R>(&mut self, name: N, rem: R)
    where
        N: Into<String>,
        R: AsRef<str>,
    {
        if self.include_slash() {
            self.push_str("/");
        }
        self.push_str("([^?]+)");
        self.push_str(&regex::escape(rem.as_ref()));
        self.named_params.push(NamedParam {
            name: name.into(),
            kind: NamedParamKind::Multi,
        });
    }

    /// Pushes a dynamic segment to the regex.
    pub fn push_dynamic_segment<N, R>(&mut self, name: N, rem: R)
    where
        N: Into<String>,
        R: AsRef<str>,
    {
        if self.include_slash() {
            self.push_str("/");
        }
        self.push_str("([^?/]+)");
        self.push_str(&regex::escape(rem.as_ref()));
        self.named_params.push(NamedParam {
            name: name.into(),
            kind: NamedParamKind::Single,
        });
    }

    /// Pushes a static segment to the regex.
    pub fn push_static_segment<S>(&mut self, segment: S)
    where
        S: AsRef<str>,
    {
        if self.include_slash() {
            self.push_str("/");
        }
        self.push_str(&regex::escape(segment.as_ref()));
    }

    /// Builds and returns the [PathRegex].
    pub fn build(mut self) -> Result<PathRegex> {
        self.regex_str += "$";
        Ok(PathRegex {
            regex: regex::Regex::new(&self.regex_str).with_context(|| "invalid path regex")?,
            named_params: self.named_params,
        })
    }
}

impl Default for PathRegexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_regex_serde_roundtrip() {
        let path_regex = super::super::build_path_regex("/api/[version]/[[...path]]").unwrap();

        let serialized = serde_json::to_string(&path_regex).unwrap();
        let deserialized: PathRegex = serde_json::from_str(&serialized).unwrap();

        assert_eq!(path_regex, deserialized);
    }
}
