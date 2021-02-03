use serde::{Deserialize, Serialize};

pub use data_rate::*;

pub mod data_rate {
    use serde::de::IntoDeserializer;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Debug, Clone, Default)]
    pub struct DataRate {
        pub spreading_factor: SpreadingFactor,
        pub bandwidth: Bandwidth,
    }

    impl Serialize for DataRate {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut str = String::new();
            str.push_str(&format!("{:?}", self.spreading_factor));
            str.push_str(&format!("{:?}", self.bandwidth));
            serializer.serialize_str(&str)
        }
    }

    impl<'de> Deserialize<'de> for DataRate {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = <&str>::deserialize(deserializer)?;
            let (sf_s, bw_s) = if s.len() > 8 {
                (&s[..4], &s[4..])
            } else {
                (&s[..3], &s[3..])
            };

            Ok(DataRate {
                bandwidth: Bandwidth::deserialize(bw_s.into_deserializer())?,
                spreading_factor: SpreadingFactor::deserialize(sf_s.into_deserializer())?,
            })
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    pub enum SpreadingFactor {
        SF7,
        SF8,
        SF9,
        SF10,
        SF11,
        SF12,
    }

    impl Default for SpreadingFactor {
        fn default() -> Self {
            SpreadingFactor::SF7
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    pub enum Bandwidth {
        BW125,
        BW250,
        BW500,
    }

    impl Default for Bandwidth {
        fn default() -> Self {
            Bandwidth::BW250
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CodingRate {
    #[serde(rename(serialize = "4/5", deserialize = "4/5"))]
    _4_5,
    #[serde(rename(serialize = "4/6", deserialize = "4/6"))]
    _4_6,
    #[serde(rename(serialize = "4/7", deserialize = "4/7"))]
    _4_7,
    #[serde(rename(serialize = "4/8", deserialize = "4/8"))]
    _4_8,
    OFF,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Modulation {
    LORA,
    FSK,
}
