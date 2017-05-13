use std::vec::IntoIter;
use std::string::ToString;

use {Error, Result};

const TOPIC_PATH_DELIMITER: char = '/';

use self::Topic::{
    Normal,
    System,
    Blank,
    SingleWildcard,
    MultiWildcard
};

/// FIXME: replace String with &str
#[derive(Debug, Clone, PartialEq)]
pub enum Topic {
    Normal(String),
    System(String), // $SYS = Topic::System("$SYS")
    Blank,
    SingleWildcard, // +
    MultiWildcard,  // #
}

impl Into<String> for Topic {
    fn into(self) -> String {
        match self {
            Normal(s) | System(s) => s,
            Blank => "".to_string(),
            SingleWildcard => "+".to_string(),
            MultiWildcard => "#".to_string()
        }
    }
}

impl Topic {
    pub fn validate(topic: &str) -> bool {
        match topic {
            "+" | "#" => true,
            _ => !(topic.contains("+") || topic.contains("#"))
        }
    }

    pub fn fit(&self, other: &Topic) -> bool {
        match *self {
            Normal(ref str) => {
                match *other {
                    Normal(ref s) => str == s,
                    SingleWildcard | MultiWildcard => true,
                    _ => false
                }
            },
            System(ref str) => {
                match *other {
                    System(ref s) => str == s,
                    _ => false
                }
            },
            Blank => {
                match *other {
                    Blank | SingleWildcard | MultiWildcard => true,
                    _ => false
                }
            },
            SingleWildcard => {
                match *other {
                    System(_) => false,
                    _ => true
                }
            },
            MultiWildcard => {
                match *other {
                    System(_) => false,
                    _ => true
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TopicPath {
    pub path: String,
    // Should be false for Topic Name
    pub wildcards: bool,
    topics: Vec<Topic>
}

impl TopicPath {
    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn get(&self, index: usize) -> Option<&Topic> {
        self.topics.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Topic> {
        self.topics.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.topics.len()
    }

    pub fn is_final(&self, index: usize) -> bool {
        let len = self.topics.len();
        len == 0 || len-1 == index
    }

    pub fn is_multi(&self, index: usize) -> bool {
        match self.topics.get(index) {
            Some(topic) => *topic == Topic::MultiWildcard,
            None => false
        }
    }

    // match with a concrete topic path
    pub fn is_match(&self, concrete: &TopicPath) -> bool {
        // argument should be a concrete path
        if concrete.wildcards {
            return false
        }

        let mut tmp = concrete.clone();
        let len = self.len();

        // if last element of self is '#', truncate concrete topic till
        // prev index of '#' in self
        // e.g 1. a/b/c/d, 2. a/b/#. convert 1 to a/b/#
        // e.g 1. z/b/c/d, 2. a/b/#. convert 1 to z/b/#
        // e.g 1. a/b 2. a/b/#. corner case: 1, 2 should match. convert 1 to a/b/#
        if let Some(v) = self.topics.last() {
            match *v {
                Topic::MultiWildcard => {
                    tmp.topics.truncate(len-1);
                    tmp.topics.push(Topic::MultiWildcard);
                }
                _ => (),
            }
        }

        // note: after this point, both the topics should be of same len
        if self.topics.len() != tmp.topics.len() {
            return false
        }

        // match till last element of self topic (self)
        // e.g a/b/c/+/e (self), a/b/c/d/e/f/g (concrete)
        // match till e of self
        for (index, topic) in self.topics.iter().enumerate() {
            if !topic.fit(&tmp.topics[index]) {
                return false
            }
        }

        true
    }

    pub fn from_str<T: AsRef<str>>(path: T) -> Result<TopicPath> {
        let mut valid = true;
        let topics: Vec<Topic> = path.as_ref().split(TOPIC_PATH_DELIMITER).map( |topic| {
            match topic {
                "+" => Topic::SingleWildcard,
                "#" => Topic::MultiWildcard,
                "" => Topic::Blank,
                _ => {
                    if !Topic::validate(topic) {
                        valid = false;
                    }
                    if topic.chars().nth(0) == Some('$') {
                        Topic::System(String::from(topic))
                    } else {
                        Topic::Normal(String::from(topic))
                    }
                }
            }
        }).collect();

        if !valid {
            return Err(Error::InvalidTopicPath);
        }

        let len = topics.len();
        let mut wildcards = false;

        for (index, topic) in topics.iter().enumerate() {
            match *topic {
                Topic::SingleWildcard => wildcards = true,
                Topic::MultiWildcard => {
                    // topics paths like a/#/c shouldn't be allowed
                    // multi wildcard is only allowed at last index
                    if index != len - 1 {
                        return Err(Error::InvalidTopicPath)
                    }
                    wildcards = true;
                }
                _ => ()
            }
        }

        Ok(TopicPath {
            path: String::from(path.as_ref()),
            topics: topics,
            wildcards: wildcards
        })
    }
}

impl IntoIterator for TopicPath {
    type Item = Topic;
    type IntoIter = IntoIter<Topic>;
    fn into_iter(self) -> Self::IntoIter {
        self.topics.into_iter()
    }
}

impl<'a> From<&'a str> for TopicPath {
    fn from(str: &'a str) -> TopicPath {
        Self::from_str(str).unwrap()
    }
}

impl From<String> for TopicPath {
    fn from(path: String) -> TopicPath {
        Self::from_str(path).unwrap()
    }
}

impl Into<String> for TopicPath {
    fn into(self) -> String {
        self.path
    }
}

pub trait ToTopicPath {
    fn to_topic_path(&self) -> Result<TopicPath>;

    fn to_topic_name(&self) -> Result<TopicPath> {
        let topic_name = try!(self.to_topic_path());
        match topic_name.wildcards {
            false => Ok(topic_name),
            true => Err(Error::TopicNameMustNotContainWildcard)
        }
    }
}

impl ToTopicPath for TopicPath {
    fn to_topic_path(&self) -> Result<TopicPath> {
        Ok(self.clone())
    }
}

impl ToTopicPath for String {
    fn to_topic_path(&self) -> Result<TopicPath> {
        TopicPath::from_str(self.clone())
    }
}

impl<'a> ToTopicPath for &'a str {
    fn to_topic_path(&self) -> Result<TopicPath> {
        TopicPath::from_str(*self)
    }
}

#[cfg(test)]
mod test {
    use super::{TopicPath, Topic};

    #[test]
    fn topic_path_test() {
        let path = "/$SYS/test/+/#";
        let topic = TopicPath::from(path);
        let mut iter = topic.into_iter();
        assert_eq!(iter.next().unwrap(), Topic::Blank);
        assert_eq!(iter.next().unwrap(), Topic::System("$SYS".to_string()));
        assert_eq!(iter.next().unwrap(), Topic::Normal("test".to_string()));
        assert_eq!(iter.next().unwrap(), Topic::SingleWildcard);
        assert_eq!(iter.next().unwrap(), Topic::MultiWildcard);
    }

    #[test]
    fn wildcards_test() {
        let topic = TopicPath::from("/a/b/c");
        assert!(!topic.wildcards);
        let topic = TopicPath::from("/a/+/c");
        assert!(topic.wildcards);
        let topic = TopicPath::from("/a/b/#");
        assert!(topic.wildcards);
    }

    #[test]
    fn topic_is_not_valid_test() {
        assert!(TopicPath::from_str("+wrong").is_err());
        assert!(TopicPath::from_str("wro#ng").is_err());
        assert!(TopicPath::from_str("w/r/o/n/g+").is_err());
        assert!(TopicPath::from_str("wrond/#/path").is_err());
    }

    #[test]
    fn match_concrete_topics_with_wildcard_topics() {
        let concrete1 = TopicPath::from_str("a/b/c").unwrap();
        let concrete2 = TopicPath::from_str("a/b/c").unwrap();

        assert!(concrete1.is_match(&concrete2));

        let concrete1 = TopicPath::from_str("a/b/c").unwrap();
        let concrete2 = TopicPath::from_str("a/b/c/d").unwrap();

        assert!(!concrete1.is_match(&concrete2));

        let concrete = TopicPath::from_str("a/b/c").unwrap();
        let wild = TopicPath::from_str("a/+/c").unwrap();

        assert!(wild.is_match(&concrete));

        let concrete = TopicPath::from_str("a/b/c").unwrap();
        let wild = TopicPath::from_str("d/e/f").unwrap();

        assert!(!wild.is_match(&concrete));

        let concrete = TopicPath::from_str("a/b/c/d/e").unwrap();
        let wild = TopicPath::from_str("a/+/c/+/e").unwrap();

        assert!(wild.is_match(&concrete));

        let concrete = TopicPath::from_str("a/b/c/d/e").unwrap();
        let wild = TopicPath::from_str("a/b/#").unwrap();

        assert!(wild.is_match(&concrete));

        let concrete = TopicPath::from_str("z/b/c/d/e").unwrap();
        let wild = TopicPath::from_str("a/b/#").unwrap();

        assert!(!wild.is_match(&concrete));

        // # should also match emptys also. these topics should match
        let concrete = TopicPath::from_str("a/b").unwrap();
        let wild = TopicPath::from_str("a/b/#").unwrap();

        assert!(wild.is_match(&concrete));


        // but '+' should'nt match emptys. these topics should'nt match
        let concrete = TopicPath::from_str("a/b").unwrap();
        let wild = TopicPath::from_str("a/b/+").unwrap();

        assert!(!wild.is_match(&concrete));
    }
}
