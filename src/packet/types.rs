use serde::{Deserialize, Serialize};

pub use data_rate::*;

pub mod data_rate {
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use std::cmp::PartialEq;
    use std::fmt::Display;
    use std::str::FromStr;
    use std::string::ToString;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct DataRate(lora_modulation::SpreadingFactor, lora_modulation::Bandwidth);

    impl Default for DataRate {
        fn default() -> Self {
            DataRate(
                lora_modulation::SpreadingFactor::_7,
                lora_modulation::Bandwidth::_250KHz,
            )
        }
    }

    impl DataRate {
        pub fn new(
            sf: lora_modulation::SpreadingFactor,
            bw: lora_modulation::Bandwidth,
        ) -> DataRate {
            DataRate(sf, bw)
        }
        pub fn spreading_factor(&self) -> lora_modulation::SpreadingFactor {
            self.0
        }
        pub fn bandwidth(&self) -> lora_modulation::Bandwidth {
            self.1
        }
    }

    impl FromStr for DataRate {
        type Err = ParseError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let (sf, bw) = if s.len() > 8 {
                (&s[..4], &s[4..])
            } else if s.len() > 3 {
                (&s[..3], &s[3..])
            } else {
                return Err(ParseError::InvalidSpreadingFactor);
            };

            Ok(DataRate(
                SmtcSpreadingFactor::from_str(sf)?.into(),
                SmtcBandwidth::from_str(bw)?.into(),
            ))
        }
    }

    impl Display for DataRate {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let smtc_sf: SmtcSpreadingFactor = self.0.into();
            let smtc_bw: SmtcBandwidth = self.1.into();
            write!(f, "{smtc_sf}{smtc_bw}")
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
            DataRate::from_str(s).map_err(de::Error::custom)
        }
    }

    #[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
    /// A local representation of the spreading factor specifically for JSON serde.
    enum SmtcSpreadingFactor {
        SF5,
        SF6,
        SF7,
        SF8,
        SF9,
        SF10,
        SF11,
        SF12,
    }

    impl From<SmtcSpreadingFactor> for lora_modulation::SpreadingFactor {
        fn from(sf: SmtcSpreadingFactor) -> lora_modulation::SpreadingFactor {
            match sf {
                SmtcSpreadingFactor::SF5 => lora_modulation::SpreadingFactor::_5,
                SmtcSpreadingFactor::SF6 => lora_modulation::SpreadingFactor::_6,
                SmtcSpreadingFactor::SF7 => lora_modulation::SpreadingFactor::_7,
                SmtcSpreadingFactor::SF8 => lora_modulation::SpreadingFactor::_8,
                SmtcSpreadingFactor::SF9 => lora_modulation::SpreadingFactor::_9,
                SmtcSpreadingFactor::SF10 => lora_modulation::SpreadingFactor::_10,
                SmtcSpreadingFactor::SF11 => lora_modulation::SpreadingFactor::_11,
                SmtcSpreadingFactor::SF12 => lora_modulation::SpreadingFactor::_12,
            }
        }
    }

    impl From<lora_modulation::SpreadingFactor> for SmtcSpreadingFactor {
        fn from(sf: lora_modulation::SpreadingFactor) -> SmtcSpreadingFactor {
            match sf {
                lora_modulation::SpreadingFactor::_5 => SmtcSpreadingFactor::SF5,
                lora_modulation::SpreadingFactor::_6 => SmtcSpreadingFactor::SF6,
                lora_modulation::SpreadingFactor::_7 => SmtcSpreadingFactor::SF7,
                lora_modulation::SpreadingFactor::_8 => SmtcSpreadingFactor::SF8,
                lora_modulation::SpreadingFactor::_9 => SmtcSpreadingFactor::SF9,
                lora_modulation::SpreadingFactor::_10 => SmtcSpreadingFactor::SF10,
                lora_modulation::SpreadingFactor::_11 => SmtcSpreadingFactor::SF11,
                lora_modulation::SpreadingFactor::_12 => SmtcSpreadingFactor::SF12,
            }
        }
    }

    impl FromStr for SmtcSpreadingFactor {
        type Err = ParseError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "SF5" => Ok(SmtcSpreadingFactor::SF5),
                "SF6" => Ok(SmtcSpreadingFactor::SF6),
                "SF7" => Ok(SmtcSpreadingFactor::SF7),
                "SF8" => Ok(SmtcSpreadingFactor::SF8),
                "SF9" => Ok(SmtcSpreadingFactor::SF9),
                "SF10" => Ok(SmtcSpreadingFactor::SF10),
                "SF11" => Ok(SmtcSpreadingFactor::SF11),
                "SF12" => Ok(SmtcSpreadingFactor::SF12),
                _ => Err(ParseError::InvalidSpreadingFactor),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
    /// A local representation of the bandwidth specifically for JSON serde.
    enum SmtcBandwidth {
        BW7,
        BW10,
        BW15,
        BW20,
        BW31,
        BW41,
        BW62,
        BW125,
        BW250,
        BW500,
    }

    impl From<SmtcBandwidth> for lora_modulation::Bandwidth {
        fn from(bw: SmtcBandwidth) -> lora_modulation::Bandwidth {
            match bw {
                SmtcBandwidth::BW7 => lora_modulation::Bandwidth::_7KHz,
                SmtcBandwidth::BW10 => lora_modulation::Bandwidth::_10KHz,
                SmtcBandwidth::BW15 => lora_modulation::Bandwidth::_15KHz,
                SmtcBandwidth::BW20 => lora_modulation::Bandwidth::_20KHz,
                SmtcBandwidth::BW31 => lora_modulation::Bandwidth::_31KHz,
                SmtcBandwidth::BW41 => lora_modulation::Bandwidth::_41KHz,
                SmtcBandwidth::BW62 => lora_modulation::Bandwidth::_62KHz,
                SmtcBandwidth::BW125 => lora_modulation::Bandwidth::_125KHz,
                SmtcBandwidth::BW250 => lora_modulation::Bandwidth::_250KHz,
                SmtcBandwidth::BW500 => lora_modulation::Bandwidth::_500KHz,
            }
        }
    }

    impl From<lora_modulation::Bandwidth> for SmtcBandwidth {
        fn from(bw: lora_modulation::Bandwidth) -> SmtcBandwidth {
            match bw {
                lora_modulation::Bandwidth::_7KHz => SmtcBandwidth::BW7,
                lora_modulation::Bandwidth::_10KHz => SmtcBandwidth::BW10,
                lora_modulation::Bandwidth::_15KHz => SmtcBandwidth::BW15,
                lora_modulation::Bandwidth::_20KHz => SmtcBandwidth::BW20,
                lora_modulation::Bandwidth::_31KHz => SmtcBandwidth::BW31,
                lora_modulation::Bandwidth::_41KHz => SmtcBandwidth::BW41,
                lora_modulation::Bandwidth::_62KHz => SmtcBandwidth::BW62,
                lora_modulation::Bandwidth::_125KHz => SmtcBandwidth::BW125,
                lora_modulation::Bandwidth::_250KHz => SmtcBandwidth::BW250,
                lora_modulation::Bandwidth::_500KHz => SmtcBandwidth::BW500,
            }
        }
    }

    impl FromStr for SmtcBandwidth {
        type Err = ParseError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "BW7" => Ok(SmtcBandwidth::BW7),
                "BW10" => Ok(SmtcBandwidth::BW10),
                "BW15" => Ok(SmtcBandwidth::BW15),
                "BW20" => Ok(SmtcBandwidth::BW20),
                "BW31" => Ok(SmtcBandwidth::BW31),
                "BW41" => Ok(SmtcBandwidth::BW41),
                "BW62" => Ok(SmtcBandwidth::BW62),
                "BW125" => Ok(SmtcBandwidth::BW125),
                "BW250" => Ok(SmtcBandwidth::BW250),
                "BW500" => Ok(SmtcBandwidth::BW500),
                _ => Err(ParseError::InvalidBandwidth),
            }
        }
    }

    impl Display for SmtcBandwidth {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{self:?}")
        }
    }

    impl Display for SmtcSpreadingFactor {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{self:?}")
        }
    }

    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum ParseError {
        #[error("String with invalid Spreading Factor")]
        InvalidSpreadingFactor,
        #[error("String with invalid Bandwidth")]
        InvalidBandwidth,
    }

    #[cfg(test)]
    mod tests {
        // Note this useful idiom: importing names from outer (for mod tests) scope.
        use super::*;
        use lora_modulation::{Bandwidth, SpreadingFactor};
        #[test]
        fn test_to_string_sf7() {
            let datarate = DataRate(SpreadingFactor::_7, Bandwidth::_500KHz);
            assert_eq!(datarate.to_string(), "SF7BW500")
        }

        #[test]
        fn test_to_string_sf10() {
            let datarate = DataRate(SpreadingFactor::_10, Bandwidth::_125KHz);
            assert_eq!(datarate.to_string(), "SF10BW125")
        }

        #[test]
        fn test_from_str_sf10() {
            let datarate = DataRate::from_str("SF10BW125").unwrap();
            assert_eq!(datarate, DataRate(SpreadingFactor::_10, Bandwidth::_125KHz))
        }

        #[test]
        fn test_from_invalid_str() {
            let datarate = DataRate::from_str("12");
            assert!(datarate.is_err())
        }

        #[test]
        fn test_from_str_sf7() {
            let datarate = DataRate::from_str("SF7BW500").unwrap();
            assert_eq!(datarate, DataRate(SpreadingFactor::_7, Bandwidth::_500KHz))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
/// A local representation of the coding rate specifically for JSON serde
enum SmtcCodingRate {
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

pub fn serialize_codr<S>(
    codr: &Option<lora_modulation::CodingRate>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let inner_cdr = match codr {
        None => SmtcCodingRate::OFF,
        Some(lora_modulation::CodingRate::_4_5) => SmtcCodingRate::_4_5,
        Some(lora_modulation::CodingRate::_4_6) => SmtcCodingRate::_4_6,
        Some(lora_modulation::CodingRate::_4_7) => SmtcCodingRate::_4_7,
        Some(lora_modulation::CodingRate::_4_8) => SmtcCodingRate::_4_8,
    };
    inner_cdr.serialize(serializer)
}

pub fn deserialize_codr<'de, D>(
    deserializer: D,
) -> Result<Option<lora_modulation::CodingRate>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let inner_cdr = SmtcCodingRate::deserialize(deserializer)?;
    Ok(match inner_cdr {
        SmtcCodingRate::OFF => None,
        SmtcCodingRate::_4_5 => Some(lora_modulation::CodingRate::_4_5),
        SmtcCodingRate::_4_6 => Some(lora_modulation::CodingRate::_4_6),
        SmtcCodingRate::_4_7 => Some(lora_modulation::CodingRate::_4_7),
        SmtcCodingRate::_4_8 => Some(lora_modulation::CodingRate::_4_8),
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Modulation {
    LORA,
    FSK,
}

pub(crate) mod base64 {
    extern crate base64;
    use crate::packet::types::base64::base64::Engine;
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&b64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(de::Error::custom)
    }
}
