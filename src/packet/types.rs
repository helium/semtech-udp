use serde::{Deserialize, Serialize};

pub use data_rate::*;

pub mod data_rate {
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use std::cmp::PartialEq;
    use std::str::FromStr;
    use std::string::ToString;
    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct DataRate(SpreadingFactor, Bandwidth);

    impl DataRate {
        pub fn new(spreading_factor: SpreadingFactor, bandwidth: Bandwidth) -> DataRate {
            DataRate(spreading_factor, bandwidth)
        }
        pub fn spreading_factor(&self) -> &SpreadingFactor {
            &self.0
        }
        pub fn bandwidth(&self) -> &Bandwidth {
            &self.1
        }
    }

    impl FromStr for DataRate {
        type Err = ParseError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let (sf, bw) = if s.len() > 8 {
                (&s[..4], &s[4..])
            } else {
                (&s[..3], &s[3..])
            };

            Ok(DataRate(
                SpreadingFactor::from_str(sf)?,
                Bandwidth::from_str(bw)?,
            ))
        }
    }

    impl ToString for DataRate {
        fn to_string(&self) -> String {
            let mut output = self.spreading_factor().to_string();
            output.push_str(&self.bandwidth().to_string());
            output
        }
    }

    impl Serialize for DataRate {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let str = self.to_string();
            serializer.serialize_str(&str)
        }
    }

    impl<'de> Deserialize<'de> for DataRate {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = <&str>::deserialize(deserializer)?;
            DataRate::from_str(&s).map_err(de::Error::custom)
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

    impl FromStr for SpreadingFactor {
        type Err = ParseError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "SF7" => Ok(SpreadingFactor::SF7),
                "SF8" => Ok(SpreadingFactor::SF8),
                "SF9" => Ok(SpreadingFactor::SF9),
                "SF10" => Ok(SpreadingFactor::SF10),
                "SF11" => Ok(SpreadingFactor::SF11),
                "SF12" => Ok(SpreadingFactor::SF12),
                _ => Err(ParseError::InvalidSpreadingFactor),
            }
        }
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

    impl FromStr for Bandwidth {
        type Err = ParseError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "BW125" => Ok(Bandwidth::BW125),
                "BW250" => Ok(Bandwidth::BW250),
                "BW500" => Ok(Bandwidth::BW500),
                _ => Err(ParseError::InvalidBandwidth),
            }
        }
    }

    impl ToString for Bandwidth {
        fn to_string(&self) -> String {
            format!("{:?}", self)
        }
    }

    impl ToString for SpreadingFactor {
        fn to_string(&self) -> String {
            format!("{:?}", self)
        }
    }

    impl Default for Bandwidth {
        fn default() -> Self {
            Bandwidth::BW250
        }
    }

    #[derive(Debug)]
    pub enum ParseError {
        InvalidSpreadingFactor,
        InvalidBandwidth,
    }
    impl std::fmt::Display for ParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            let msg = match self {
                ParseError::InvalidSpreadingFactor => "Invalid spreading factor input",
                ParseError::InvalidBandwidth => "Invalid bandwidth input",
            };
            write!(f, "{}", msg)
        }
    }

    #[cfg(test)]
    mod tests {
        // Note this useful idiom: importing names from outer (for mod tests) scope.
        use super::*;

        #[test]
        fn test_to_string_sf7() {
            let datarate = DataRate(SpreadingFactor::SF7, Bandwidth::BW500);
            assert_eq!(datarate.to_string(), "SF7BW500")
        }

        #[test]
        fn test_to_string_sf10() {
            let datarate = DataRate(SpreadingFactor::SF10, Bandwidth::BW125);
            assert_eq!(datarate.to_string(), "SF10BW125")
        }

        #[test]
        fn test_from_str_sf10() {
            let datarate = DataRate::from_str("SF10BW125").unwrap();
            assert_eq!(datarate, DataRate(SpreadingFactor::SF10, Bandwidth::BW125))
        }

        #[test]
        fn test_from_str_sf7() {
            let datarate = DataRate::from_str("SF7BW500").unwrap();
            assert_eq!(datarate, DataRate(SpreadingFactor::SF7, Bandwidth::BW500))
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

pub(crate) mod base64 {
    extern crate base64;
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&base64::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        base64::decode(s).map_err(de::Error::custom)
    }
}
