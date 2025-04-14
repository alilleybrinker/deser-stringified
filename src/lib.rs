use serde::{
    Deserializer,
    de::{Deserialize, DeserializeOwned, Visitor},
};

/// deserialize a stringified field embedded in other data using a custom
/// parser, allowing you to parse JSON, YAML, or any other serde-compatible
/// format.
pub fn deser_stringified_format<'de, D, T, F, E>(deserializer: D, parser: F) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
    F: Fn(&'de str) -> Result<T, E>,
    E: std::fmt::Display,
{
    struct StringifiedFormatVisitor<T, F, E> {
        parser: F,
        marker: std::marker::PhantomData<T>,
        error_marker: std::marker::PhantomData<E>,
    }

    impl<'de, T, F, E> Visitor<'de> for StringifiedFormatVisitor<T, F, E>
    where
        T: Deserialize<'de>,
        F: Fn(&'de str) -> Result<T, E>,
        E: std::fmt::Display,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "a string containing formatted data that parses to type {}",
                std::any::type_name::<T>()
            )
        }

        /// Avoid allocating if we can get a slice of the string from the Deserializer.
        fn visit_borrowed_str<A>(self, value: &'de str) -> Result<Self::Value, A>
        where
            A: serde::de::Error,
        {
            eprintln!("visiting borrowed string");
            (self.parser)(value).map_err(A::custom)
        }
    }

    let visitor = StringifiedFormatVisitor {
        parser,
        marker: std::marker::PhantomData,
        error_marker: std::marker::PhantomData,
    };

    deserializer.deserialize_str(visitor)
}

/// deserialize a stringified field embedded in other data using a custom
/// parser, allowing you to parse JSON, YAML, or any other serde-compatible
/// format.
pub fn deser_stringified_format_owned<'de, D, T, F, E>(
    deserializer: D,
    parser: F,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
    F: Fn(&str) -> Result<T, E>,
    E: std::fmt::Display,
{
    struct StringifiedFormatVisitor<T, F, E> {
        parser: F,
        marker: std::marker::PhantomData<T>,
        error_marker: std::marker::PhantomData<E>,
    }

    impl<'de, T, F, E> Visitor<'de> for StringifiedFormatVisitor<T, F, E>
    where
        T: Deserialize<'de>,
        F: Fn(&str) -> Result<T, E>,
        E: std::fmt::Display,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "a string containing formatted data that parses to type {}",
                std::any::type_name::<T>()
            )
        }

        /// Clone the string ourselves if the Deserializer doesn't take ownership.
        fn visit_str<A>(self, value: &str) -> Result<Self::Value, A>
        where
            A: serde::de::Error,
        {
            (self.parser)(value).map_err(A::custom)
        }

        /// Move an already-allocated string if the Deserializer does take ownership.
        fn visit_string<A>(self, value: String) -> Result<Self::Value, A>
        where
            A: serde::de::Error,
        {
            (self.parser)(&value).map_err(A::custom)
        }
    }

    let visitor = StringifiedFormatVisitor {
        parser,
        marker: std::marker::PhantomData,
        error_marker: std::marker::PhantomData,
    };

    deserializer.deserialize_string(visitor)
}

#[cfg(feature = "serde_json")]
pub fn deser_stringified_json<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    deser_stringified_format(deserializer, serde_json::from_str)
}

#[cfg(feature = "serde_json")]
pub fn deser_stringified_json_owned<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: DeserializeOwned,
{
    deser_stringified_format_owned(deserializer, |s| serde_json::from_str(s))
}

#[cfg(feature = "serde_yaml")]
pub fn deser_stringified_yaml<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    deser_stringified_format(deserializer, serde_yaml::from_str)
}

#[cfg(feature = "serde_yaml")]
pub fn deser_stringified_yaml_owned<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: DeserializeOwned,
{
    deser_stringified_format_owned(deserializer, |s| serde_yaml::from_str(s))
}

#[cfg(feature = "toml")]
pub fn deser_stringified_toml_owned<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    deser_stringified_format_owned(deserializer, toml::from_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Metadata {
        key: i32,
        enabled: bool,
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn json_parsing() {
        #[derive(Deserialize)]
        #[serde(bound = "'de: 'inner")]
        struct Example<'inner> {
            #[serde(deserialize_with = "deser_stringified_json")]
            data: Inner<'inner>,
        }

        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Inner<'inner> {
            key: i64,
            enabled: bool,
            name: &'inner str,
        }

        let json_str = r#"{"data": "{\"key\": 1, \"enabled\": false, \"name\": \"Jim\"}"}"#;
        let parsed: Example = serde_json::from_str(json_str).unwrap();
        assert_eq!(
            parsed.data,
            Inner {
                key: 1,
                enabled: false,
                name: "Jim"
            }
        );
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn json_parsing_owned() {
        #[derive(Deserialize)]
        struct Example<T>
        where
            T: DeserializeOwned,
        {
            #[serde(deserialize_with = "deser_stringified_json_owned")]
            data: T,
        }

        let json_str = r#"{"data": "{\"key\": 1, \"enabled\": false}"}"#;
        let parsed: Example<serde_json::Value> = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed.data, serde_json::json!({"key": 1, "enabled": false}));

        let struct_parsed: Example<Metadata> = serde_json::from_str(json_str).unwrap();
        assert_eq!(struct_parsed.data.key, 1);
        assert_eq!(struct_parsed.data.enabled, false);
    }

    #[test]
    #[cfg(feature = "serde_yaml")]
    fn yaml_parsing() {
        #[derive(Deserialize)]
        struct Example<T>
        where
            T: for<'inner> Deserialize<'inner>,
        {
            #[serde(deserialize_with = "deser_stringified_yaml")]
            data: T,
        }

        let yaml_str = r#"
data: |
  key: 1
  enabled: false
"#;
        let parsed: Example<serde_yaml::Value> = serde_yaml::from_str(yaml_str).unwrap();

        let mut expected_mapping = serde_yaml::Mapping::new();
        expected_mapping.insert(
            serde_yaml::Value::String("key".to_string()),
            serde_yaml::Value::Number(1.into()),
        );
        expected_mapping.insert(
            serde_yaml::Value::String("enabled".to_string()),
            serde_yaml::Value::Bool(false),
        );
        let expected = serde_yaml::Value::Mapping(expected_mapping);
        assert_eq!(parsed.data, expected);

        let struct_parsed: Example<Metadata> = serde_yaml::from_str(yaml_str).unwrap();
        assert_eq!(struct_parsed.data.key, 1);
        assert_eq!(struct_parsed.data.enabled, false);
    }

    #[test]
    #[cfg(feature = "toml")]
    fn toml_parsing() {
        #[derive(Deserialize)]
        struct Example<T: serde::de::DeserializeOwned> {
            #[serde(deserialize_with = "deser_stringified_toml_owned")]
            data: T,
        }

        let toml_str = "data = \"\"\"\nkey = 1\nenabled = false\n\"\"\"";
        let parsed: Example<toml::Value> = toml::from_str(toml_str).unwrap();

        let mut expected = toml::value::Table::new();
        expected.insert("key".to_string(), toml::Value::Integer(1));
        expected.insert("enabled".to_string(), toml::Value::Boolean(false));
        assert_eq!(parsed.data, toml::Value::Table(expected));

        let struct_parsed: Example<Metadata> = toml::from_str(toml_str).unwrap();
        assert_eq!(struct_parsed.data.key, 1);
        assert_eq!(struct_parsed.data.enabled, false);
    }
}
