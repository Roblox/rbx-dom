use std::convert::TryFrom;
use std::str::FromStr;
use std::string::FromUtf8Error;

#[derive(Default, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct Tags {
    tag_list: Vec<String>,
}

impl Tags {
    pub fn new() -> Tags {
        Self {
            tag_list: Vec::new(),
        }
    }

    pub fn as_vec(&self) -> &Vec<String> {
        &self.tag_list
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<String> {
        &mut self.tag_list
    }
}

impl From<Vec<String>> for Tags {
    fn from(tag_list: Vec<String>) -> Tags {
        Self { tag_list }
    }
}

impl From<Vec<&str>> for Tags {
    fn from(tag_list: Vec<&str>) -> Tags {
        Self {
            tag_list: tag_list
                .iter()
                .map(|tag_name| String::from_str(tag_name).unwrap())
                .collect(),
        }
    }
}

impl From<&Tags> for Vec<u8> {
    fn from(tags: &Tags) -> Vec<u8> {
        let tag_list = &tags.tag_list;
        let mut buf: Vec<u8> = Vec::with_capacity(tag_list.capacity() + tag_list.len());

        for tag_name in tag_list.as_slice() {
            buf.extend_from_slice(tag_name.as_bytes());
            buf.extend_from_slice(b"\0")
        }

        buf
    }
}

impl TryFrom<Vec<u8>> for Tags {
    type Error = FromUtf8Error;

    fn try_from(buf: Vec<u8>) -> Result<Tags, FromUtf8Error> {
        let len = buf.len();
        let mut tag_list = Vec::new();
        let mut count = 0;

        while count < len {
            let tag_name: Vec<u8> = buf
                .iter()
                .skip(count)
                .copied()
                .take_while(|element| {
                    count += 1;
                    *element != 0
                })
                .collect();

            tag_list.push(String::from_utf8(tag_name)?);
        }

        Ok(Self { tag_list })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg(feature = "serde")]
    fn serialization() {
        let serialized = "[\"is\",\"that\",\"your\",\"grandma's\",\"coat?\"]";
        let value = Tags::from(vec!["is", "that", "your", "grandma's", "coat?"]);

        assert_eq!(serialized, serde_json::to_string(&value).unwrap());
        assert_eq!(value, serde_json::from_str::<Tags>(serialized).unwrap());
    }

    #[test]
    fn from_binary_string() {
        let value = b"ez\0pz".to_vec();

        assert_eq!(*(Tags::try_from(value).unwrap().as_vec()), vec!["ez", "pz"])
    }
}
