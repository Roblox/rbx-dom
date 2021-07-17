use std::string::FromUtf8Error;

#[derive(Default, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct Tags {
    members: Vec<String>,
}

impl Tags {
    pub fn new() -> Tags {
        Self {
            members: Vec::new(),
        }
    }

    pub fn as_slice(&self) -> &[String] {
        &self.members
    }

    pub fn as_mut_slice(&mut self) -> &mut [String] {
        &mut self.members
    }

    pub fn decode(buf: Vec<u8>) -> Result<Self, FromUtf8Error> {
        Ok(buf
            .split(|element| *element == 0)
            .map(|tag_name| String::from_utf8(tag_name.to_vec()))
            .collect::<Result<Vec<String>, _>>()?
            .into())
    }
}

impl From<Vec<String>> for Tags {
    fn from(members: Vec<String>) -> Tags {
        Self { members }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg(feature = "serde")]
    fn serialization() {
        let serialized = "[\"is\",\"that\",\"your\",\"grandma's\",\"coat?\"]";
        let tags = vec!["is", "that", "your", "grandma's", "coat?"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>();
        let value = Tags::from(tags);

        assert_eq!(serialized, serde_json::to_string(&value).unwrap());
        assert_eq!(value, serde_json::from_str::<Tags>(serialized).unwrap());
    }

    #[test]
    fn from_binary_string() {
        let value = b"ez\0pz".to_vec();

        assert_eq!(Tags::decode(value).unwrap().as_slice(), &["ez", "pz"])
    }
}
