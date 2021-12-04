pub mod duration_ms {
    use serde::{de, Serializer};
    use std::{fmt, time::Duration};

    pub struct DurationVisitor;
    impl<'de> de::Visitor<'de> for DurationVisitor {
        type Value = Duration;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a milliseconds represents std::time::Duration")
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Duration::from_millis(v))
        }
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Duration::from_millis(v.max(0) as u64))
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Duration, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        d.deserialize_u64(DurationVisitor)
    }

    pub fn serialize<S>(x: &Duration, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_u64(x.as_millis() as u64)
    }
}

pub mod duration_second {
    use chrono::Duration;
    use serde::{de, Deserialize, Serializer};

    /// Deserialize from seconds (represented as u64)
    pub fn deserialize<'de, D>(d: D) -> Result<Duration, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let duration: i64 = Deserialize::deserialize(d)?;
        Ok(Duration::seconds(duration))
    }

    /// Serialize to seconds (represented as u64)
    pub fn serialize<S>(x: &Duration, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_i64(x.num_seconds())
    }
}

pub mod space_separated_scopes {
    use serde::{de, Deserialize, Serializer};
    use std::collections::HashSet;

    pub fn deserialize<'de, D>(d: D) -> Result<HashSet<String>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let scopes: &str = Deserialize::deserialize(d)?;
        Ok(scopes.split_whitespace().map(|x| x.to_owned()).collect())
    }

    pub fn serialize<S>(scopes: &HashSet<String>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let scopes = scopes.clone().into_iter().collect::<Vec<_>>().join(" ");
        s.serialize_str(&scopes)
    }
}

// pub mod option_duration_ms {
//     use serde::{de, Serializer};
//     use std::{fmt, time::Duration};

//     struct OptionDurationVisitor;

//     impl<'de> de::Visitor<'de> for OptionDurationVisitor {
//         type Value = Option<Duration>;

//         fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//             write!(
//                 formatter,
//                 "a optional milliseconds represents std::time::Duration"
//             )
//         }

//         fn visit_none<E>(self) -> Result<Self::Value, E>
//         where
//             E: de::Error,
//         {
//             Ok(None)
//         }

//         fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
//         where
//             D: de::Deserializer<'de>,
//         {
//             Ok(Some(
//                 deserializer.deserialize_u64(duration_ms::DurationVisitor)?,
//             ))
//         }
//     }

//     pub fn deserialize<'de, D>(d: D) -> Result<Option<Duration>, D::Error>
//     where
//         D: de::Deserializer<'de>,
//     {
//         d.deserialize_option(OptionDurationVisitor)
//     }

//     pub fn serialize<S>(x: &Option<Duration>, s: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         match *x {
//             Some(duration) => s.serialize_u64(duration.as_millis() as u64),
//             None => s.serialize_none(),
//         }
//     }
// }
